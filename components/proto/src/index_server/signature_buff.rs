use byteorder::{BigEndian, WriteBytesExt};
use common::canonical_serialize::CanonicalSerialize;
use common::int_convert::usize_to_u64;
use crypto::hash;
use crypto::identity::verify_signature;

use super::messages::{IndexMutation, MutationsUpdate, UpdateFriend};

// Canonical Serialization (To be used for signatures):
// ----------------------------------------------------

impl CanonicalSerialize for UpdateFriend {
    fn canonical_serialize(&self) -> Vec<u8> {
        let mut res_bytes = Vec::new();
        res_bytes.extend_from_slice(&self.public_key);
        res_bytes
            .write_u128::<BigEndian>(self.send_capacity)
            .unwrap();
        res_bytes
            .write_u128::<BigEndian>(self.recv_capacity)
            .unwrap();
        res_bytes
    }
}

impl CanonicalSerialize for IndexMutation {
    fn canonical_serialize(&self) -> Vec<u8> {
        let mut res_bytes = Vec::new();
        match self {
            IndexMutation::UpdateFriend(update_friend) => {
                res_bytes.push(0);
                res_bytes.extend(update_friend.canonical_serialize());
            }
            IndexMutation::RemoveFriend(public_key) => {
                res_bytes.push(1);
                res_bytes.extend_from_slice(public_key);
            }
        };
        res_bytes
    }
}

pub const MUTATIONS_UPDATE_PREFIX: &[u8] = b"MUTATIONS_UPDATE";

impl MutationsUpdate {
    pub fn signature_buff(&self) -> Vec<u8> {
        let mut res_bytes = Vec::new();
        res_bytes.extend_from_slice(&hash::sha_512_256(MUTATIONS_UPDATE_PREFIX));
        res_bytes.extend_from_slice(&self.node_public_key);

        res_bytes
            .write_u64::<BigEndian>(usize_to_u64(self.index_mutations.len()).unwrap())
            .unwrap();
        for mutation in &self.index_mutations {
            res_bytes.extend(mutation.canonical_serialize());
        }

        res_bytes.extend_from_slice(&self.time_hash);
        res_bytes.extend_from_slice(&self.session_id);
        res_bytes.write_u64::<BigEndian>(self.counter).unwrap();
        res_bytes.extend_from_slice(&self.rand_nonce);

        res_bytes
    }

    /// Verify the signature at the MutationsUpdate structure.
    /// Note that this structure also contains the `node_public_key` field, which is the identity
    /// of the node who signed this struct.
    pub fn verify_signature(&self) -> bool {
        let signature_buff = self.signature_buff();
        verify_signature(&signature_buff, &self.node_public_key, &self.signature)
    }
}
