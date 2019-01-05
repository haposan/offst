mod handle_control;
mod handle_friend;
mod handle_liveness;
mod handle_init;
mod sender;
mod canceler;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::collections::HashMap;
use identity::IdentityClient;

use crypto::uid::Uid;
use crypto::identity::PublicKey;
use crypto::crypto_rand::CryptoRandom;

use common::canonical_serialize::CanonicalSerialize;
use proto::funder::messages::FunderOutgoingControl;

use super::state::{FunderState, FunderMutation};
use self::handle_control::{HandleControlError};
use self::handle_friend::HandleFriendError;
use self::handle_liveness::HandleLivenessError;
use self::handle_init::handle_init;
use self::sender::SendCommands;
use super::types::{FunderIncoming,
    FunderOutgoingComm, FunderIncomingComm};
use super::ephemeral::{Ephemeral, EphemeralMutation};
use super::friend::{FriendState, ChannelStatus};
use super::report::{funder_mutation_to_report_mutations, 
    ephemeral_mutation_to_report_mutations};


pub struct MutableFunderState<A> {
    state: FunderState<A>,
    mutations: Vec<FunderMutation<A>>,
}

impl<A> MutableFunderState<A> 
where
    A: Clone,
{
    pub fn new(state: FunderState<A>) -> Self {
        MutableFunderState {
            state,
            mutations: Vec::new(),
        }
    }

    pub fn mutate(&mut self, mutation: FunderMutation<A>) {
        self.state.mutate(&mutation);
        self.mutations.push(mutation);
    }

    pub fn state(&self) -> &FunderState<A> {
        &self.state
    }

    pub fn done(self) -> (FunderState<A>, Vec<FunderMutation<A>>) {
        (self.state, self.mutations)
    }
}

pub struct MutableEphemeral {
    ephemeral: Ephemeral,
    mutations: Vec<EphemeralMutation>,
}

impl MutableEphemeral {
    pub fn new(ephemeral: Ephemeral) -> Self {
        MutableEphemeral {
            ephemeral,
            mutations: Vec::new(),
        }
    }
    pub fn mutate(&mut self, mutation: EphemeralMutation) {
        self.ephemeral.mutate(&mutation);
        self.mutations.push(mutation);
    }

    pub fn ephemeral(&self) -> &Ephemeral {
        &self.ephemeral
    }

    pub fn done(self) -> (Ephemeral, Vec<EphemeralMutation>) {
        (self.ephemeral, self.mutations)
    }
}

#[derive(Debug)]
pub enum FunderHandlerError {
    HandleControlError(HandleControlError),
    HandleFriendError(HandleFriendError),
    HandleLivenessError(HandleLivenessError),
}

pub struct FunderHandlerOutput<A: Clone> {
    pub funder_mutations: Vec<FunderMutation<A>>,
    pub ephemeral_mutations: Vec<EphemeralMutation>,
    pub outgoing_comms: Vec<FunderOutgoingComm<A>>,
    pub outgoing_control: Vec<FunderOutgoingControl<A>>,
}


pub struct MutableFunderHandler<A:Clone,R> {
    // TODO: Is there a more elegant way to do this, instead of having two states?
    // Does ephemeral require an initial ephemeral too?
    initial_state: FunderState<A>,
    state: FunderState<A>,
    ephemeral: Ephemeral,
    pub identity_client: IdentityClient,
    pub rng: R, 
    send_commands: SendCommands,
    max_operations_in_batch: usize,
    funder_mutations: Vec<FunderMutation<A>>,
    ephemeral_mutations: Vec<EphemeralMutation>,
    outgoing_comms: Vec<FunderOutgoingComm<A>>,
    outgoing_control: Vec<FunderOutgoingControl<A>>,
}

impl<A,R> MutableFunderHandler<A,R> 
where
    A: CanonicalSerialize + Clone + Debug + PartialEq + Eq + 'static,
{
    /*
    pub fn state(&self) -> &FunderState<A> {
        &self.state
    }
    */

    fn get_friend(&self, friend_public_key: &PublicKey) -> Option<&FriendState<A>> {
        self.state.friends.get(&friend_public_key)
    }

    pub fn done(self) -> FunderHandlerOutput<A> {
        let mut outgoing_control = self.outgoing_control;
        // Create differential report according to mutations:
        let mut report_mutations = Vec::new();
        let mut running_state = self.initial_state.clone();
        for funder_mutation in &self.funder_mutations {
            report_mutations.extend(funder_mutation_to_report_mutations(funder_mutation, &running_state));
            running_state.mutate(funder_mutation);
        }
        
        for ephemeral_mutation in &self.ephemeral_mutations {
            report_mutations.extend(ephemeral_mutation_to_report_mutations(ephemeral_mutation));
        }

        if !report_mutations.is_empty() {
            outgoing_control.push(FunderOutgoingControl::ReportMutations(report_mutations));
        }

        FunderHandlerOutput {
            funder_mutations: self.funder_mutations,
            ephemeral_mutations: self.ephemeral_mutations,
            outgoing_comms: self.outgoing_comms,
            outgoing_control: outgoing_control,
        }
    }

    /// Apply an Ephemeral mutation and also remember it.
    pub fn apply_ephemeral_mutation(&mut self, mutation: EphemeralMutation) {
        self.ephemeral.mutate(&mutation);
        self.ephemeral_mutations.push(mutation);
    }

    /// Apply a Funder mutation and also remember it.
    pub fn apply_funder_mutation(&mut self, mutation: FunderMutation<A>) {
        self.state.mutate(&mutation);
        self.funder_mutations.push(mutation);
    }

    pub fn add_outgoing_comm(&mut self, outgoing_comm: FunderOutgoingComm<A>) {
        self.outgoing_comms.push(outgoing_comm);
    }

    pub fn add_outgoing_control(&mut self, outgoing_control: FunderOutgoingControl<A>) {
        self.outgoing_control.push(outgoing_control);
    }

    /*
    pub fn add_response_received(&mut self, response_received: ResponseReceived) {
        self.add_outgoing_control(FunderOutgoingControl::ResponseReceived(response_received));
    }
    */

    /// Find the originator of a pending local request.
    /// This should be a pending remote request at some other friend.
    /// Returns the public key of a friend. If we are the origin of this request, the function returns None.
    ///
    /// TODO: We need to change this search to be O(1) in the future. Possibly by maintaining a map
    /// between request_id and (friend_public_key, friend).
    pub fn find_request_origin(&self, request_id: &Uid) -> Option<&PublicKey> {
        for (friend_public_key, friend) in &self.state.friends {
            match &friend.channel_status {
                ChannelStatus::Inconsistent(_) => continue,
                ChannelStatus::Consistent(token_channel) => {
                    if token_channel
                        .get_mutual_credit()
                        .state()
                        .pending_requests
                        .pending_remote_requests
                        .contains_key(request_id) {
                            return Some(friend_public_key)
                    }
                },
            }
        }
        None
    }

    /// Is it a good idea to forward requests to this friend at this moment?
    /// This checks if it is likely that the friend will answer in a timely manner.
    pub fn is_friend_ready(&self, friend_public_key: &PublicKey) -> bool {
        let friend = self.get_friend(friend_public_key).unwrap();
        if !self.ephemeral.liveness.is_online(friend_public_key) {
            return false;
        }

        // Make sure that the channel is consistent:
        let token_channel = match &friend.channel_status {
            ChannelStatus::Inconsistent(_) => return false,
            ChannelStatus::Consistent(token_channel) => token_channel,
        };

        // Make sure that the remote side has open requests:
        token_channel
            .get_mutual_credit()
            .state()
            .requests_status.remote
            .is_open()
    }

}

fn gen_mutable<A:Clone + Debug, R: CryptoRandom>(identity_client: IdentityClient,
                       rng: R,
                       funder_state: &FunderState<A>,
                       funder_ephemeral: &Ephemeral,
                       max_operations_in_batch: usize) -> MutableFunderHandler<A,R> {

    MutableFunderHandler {
        initial_state: funder_state.clone(),
        state: funder_state.clone(),
        ephemeral: funder_ephemeral.clone(),
        identity_client,
        rng,
        send_commands: SendCommands::new(),
        max_operations_in_batch,
        funder_mutations: Vec::new(),
        ephemeral_mutations: Vec::new(),
        outgoing_comms: Vec::new(),
        outgoing_control: Vec::new(),
    }
}

pub async fn funder_handle_message<A,R>(
                      identity_client: IdentityClient,
                      rng: R,
                      funder_state: FunderState<A>,
                      funder_ephemeral: Ephemeral,
                      funder_incoming: FunderIncoming<A>,
                      max_operations_in_batch: usize) 
        -> Result<FunderHandlerOutput<A>, FunderHandlerError> 
where
    A: CanonicalSerialize + Clone + Debug + PartialEq + Eq + 'static,
    R: CryptoRandom + 'static,
{

    let mut m_state = MutableFunderState::new(funder_state);
    let mut m_ephemeral = MutableEphemeral::new(funder_ephemeral);
    let mut outgoing_comms = Vec::new();
    let mut outgoing_control = Vec::new();
    let mut send_commands = SendCommands::new();

    match funder_incoming {
        FunderIncoming::Init =>  {
            let channeler_configs = handle_init(&m_state);
            for channeler_config in channeler_configs {
                outgoing_comms.push(FunderOutgoingComm::ChannelerConfig(channeler_config));
            }
        },
        FunderIncoming::Control(control_message) =>
            handle_control_message(control_message)
                .map_err(FunderHandlerError::HandleControlError)?,
        FunderIncoming::Comm(incoming_comm) => {
            match incoming_comm {
                FunderIncomingComm::Liveness(liveness_message) =>
                    handle_liveness_message(liveness_message)
                        .map_err(FunderHandlerError::HandleLivenessError)?,
                FunderIncomingComm::Friend((origin_public_key, friend_message)) => 
                    handle_friend_message(&origin_public_key, friend_message)
                        .map_err(FunderHandlerError::HandleFriendError)?,
            }
        },
    };

    // Send all possible messages according to SendCommands
    let friend_messages = await!(create_friend_messages(m_state,
                                  m_ephemeral,
                                  send_commands,
                                  max_operations_in_batch,
                                  identity_client,
                                  rng));

    for friend_message in friend_messages {
        outgoing_comms.push(FunderOutgoingComm::FriendMessage(outgoing_message));
    }

    Ok(FunderHandlerOutput {
        funder_mutations,
        ephemeral_mutations,
        outgoing_comms,
        outgoing_control,
    })
}

