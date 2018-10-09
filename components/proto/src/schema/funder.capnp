@0xe7603b9ac00e2251;

using import "common.capnp".CustomUInt128;
using import "common.capnp".CustomUInt256;
using import "common.capnp".CustomUInt512;
using import "common.capnp".Ratio128;


# Token channel messages
# ----------------------

struct FriendMoveToken {
        operations @0: List(FriendOperation);
        oldToken @1: CustomUInt256;
        randNonce @2: CustomUInt128;
}

# Acknowledge that a FriendMoveToken message was received.
# Until this message is sent, the remote side will keep resending the
# last FriendMoveToken message.
struct FriendMoveTokenAck {
        ackedToken @0: CustomUInt256;
}

# Request the token from remote side.
struct FriendRequestToken {
        lastToken @0: CustomUInt256;
        # Last token we have sent to the remote side.
        # This allows the remote side to ignore this message if it arrived too
        # late.
}


struct FriendInconsistencyError {
        optAck :union {
                resetToken @0: Data;
                none @1: Void;
        }
        currentToken @2: CustomUInt256;
        balanceForReset @3: CustomUInt128;
        # Note that this is actually a signed number (Highest bit is the sign
        # bit, Two's complement method). TODO: Should we have a separate type,
        # like CustomInt128?
}

struct FriendMessage {
        union {
                friendMoveToken @0: FriendMoveToken;
                friendMoveTokenAck @1: FriendMoveTokenAck;
                friendRequestToken @2: FriendRequestToken;
                friendInconsistencyError @3: FriendInconsistencyError;
                friendKeepAlive @4: Void;
        }
}




# Token Operations
# ------------------

# Set the maximum possible debt for the remote party.
# Note: It is not possible to set a maximum debt smaller than the current debt
# This will cause an inconsistency.
struct SetRemoteMaxDebtOp {
        remoteMaxDebt @0: UInt64;
}

struct FriendsRoute {
        nodePublicKeys @0: List(CustomUInt256);
        # A list of public keys
}

struct FriendFreezeLink {
        sharedCredits @0: UInt64;
        # Credits shared for freezing through previous edge.
        usableRatio @1: Ratio128;
        # Ratio of credits that can be used for freezing from the previous
        # edge. Ratio might only be an approximation to real value, if the real
        # value can not be represented as a u128/u128.
}


struct RequestSendFundsOp { 
        requestId @0: CustomUInt128;
        route @1: FriendsRoute;
        destPayment @2: CustomUInt128;
        invoiceId @3: CustomUInt256;
        freezeLinks @4: List(FriendFreezeLink);
        # Variable amount of freezing links. This is used for protection
        # against DoS of credit freezing by have exponential decay of available
        # credits freezing according to derived trust.
        # This part should not be signed in the Response message.
}

struct ResponseSendFundsOp {
        requestId @0: CustomUInt128;
        randNonce @1: CustomUInt128;
        signature @2: CustomUInt512;
        # Signature{key=recipientKey}(
        #   sha512/256("FUND_SUCCESS") ||
        #   sha512/256(requestId || sha512/256(route) || randNonce) ||
        #   destPayment ||
        #   invoiceId
        # )
        #
        # Note that the signature contains an inner blob (requestId || ...).
        # This is done to make the size of the receipt shorter.
        # See also the Receipt structure.
}

struct FailureSendFundsOp {
        requestId @0: CustomUInt128;
        reportingPublicKey @1: CustomUInt256;
        # Index of the reporting node in the route of the corresponding request.
        # The reporting node cannot be the destination node.
        randNonce @2: CustomUInt128;
        signature @3: CustomUInt512;
        # Signature{key=recipientKey}(
        #   sha512/256("FUND_FAILURE") ||
        #   requestId ||
        #   sha512/256(route) || 
        #   destPayment ||
        #   invoiceId ||
        #   reportingPublicKey ||
        #   randNonce
        # )
}


struct FriendOperation {
        union {
                enableRequests @0: Void;
                disableRequests @1: Void;
                setRemoteMaxDebt @2: SetRemoteMaxDebtOp;
                requestSendFunds @3: RequestSendFundsOp;
                responseSendFunds @4: ResponseSendFundsOp;
                failureSendFunds @5: FailureSendFundsOp;
        }
}
