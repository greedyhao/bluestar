use std::fmt;

const L2CAP_DEFAULT_MTU: u16 = 625;

/// The state of a L2CAP channel, according to
/// BLUETOOTH CORE SPECIFICATION Version 5.3 | Vol 3, Part A, page 1088
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Closed,
    WaitConnect,
    WaitConnectRsp,
    Config,
    WatiDisconnect,

    WillSendConnectReq,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            State::Closed => write!(f, "CLOSED"),
            State::WaitConnect => write!(f, "WaitConnect"),
            State::WaitConnectRsp => write!(f, "WaitConnectRsp"),
            State::Config => write!(f, "Config"),
            State::WatiDisconnect => write!(f, "WatiDisconnect"),
            State::WillSendConnectReq => write!(f, "WillSendConnectReq"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Substate {
    WaitConfig,
    WaitSendConfig,
    WaitConfigReqRsp,
    WaitConfigRsp,
    WaitConfigReq,
    WaitIndFinalRsp,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum InternalEvent {
    OpenChannelReq,
    OpenChannelRsp,
    ConfigureChannelReq,
    CloseChannelReq,
    SendDateReq,
    ReconfigureChannelReq,
    ControllerLogicalLinkInd,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SignalingCommand {
    CommandRejectRsp = 0x01,
    ConnectionReq,
    ConnectionRsp,
    ConfigurationReq,
    ConfigurationRsp,
    DisconnectionReq,
    DisconnectionRsp,
    EchoReq,
    EchoRsp,
    InformationReq,
    InformationRsp,

    ConnectionParameterUpdateReq = 0x12,
    ConnectionParameterUpdateRsp,
    LeCreditBasedConnectionReq,
    LeCreditBasedConnectionRsp,
    FlowControlCreditInd,
    CreditBasedConnectionReq,
    CreditBasedConnectionRsp,
    CreditBasedReconnectionReq,
    CreditBasedReconnectionRsp,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SignalRejectReason {
    CommandNotUnderstood = 0x0000,
    SignalingMTUExceeded,
    InvalidCIDInRequest,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SignalConnectionResult {
    Successful = 0x0000,
    Pending,
    RefusedPSMNotSupported,
    RefusedSecurityBlock,
    RefusedNoResourcesAvaliable,
    RefusedInvalidSourceCID,
    RefusedSourceCIDAlreadyAllocated,
}

/// Only defined for Result = Pending
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SignalConnectionStatus {
    NoFurtherInformationAvaliable = 0x0000,
    AuthenticationPending,
    Authorization_Pending,
}

enum SignalConfigurationResult {
    Successful = 0x0000,
    FailureUnacceptableParamters,
    FailureRejectd,
    FailureUnknownOptions,
    Pending,
    FailureFlowSpecRejected,
}

type BtDevAddr = [u8; 6];

#[derive(Debug, Clone)]
pub struct Channel {
    state: State,
    sub_state: Substate,
    addr: BtDevAddr,

    local_cid: u16,
    remote_cid: u16,

    local_mtu: u16,
    remote_mtu: u16,

    /// next signaling sequence number
    sig_seq_num: u8,

    /// Protocol/Service Multiplexer
    psm: u16,
}

impl Channel {
    pub fn new(psm: u16) -> Channel {
        Channel {
            state: State::WillSendConnectReq,
            sub_state: Substate::WaitConfig,
            addr: [0; 6],

            local_cid: 0,
            remote_cid: 0,

            local_mtu: 0,
            remote_mtu: L2CAP_DEFAULT_MTU,

            sig_seq_num: 0,

            psm,
        }
    }

    pub fn request(&mut self, data: &[u8]) {
        // TODO: to hci
    }
    pub fn confirm(&mut self, data: &[u8]) {
        self.run();
    }
    pub fn response(&mut self, data: &[u8]) {
        // TODO: to hci
    }
    pub fn indication(&mut self, data: &[u8]) {
        self.run();
    }

    fn run(&mut self) {
        self.run_for_classic_channel();
    }

    fn run_for_classic_channel(&mut self) {
        match self.state {
            State::WillSendConnectReq => {
                self.state = State::WaitConnectRsp;
                self.send_classic_signaling_packet(SignalingCommand::ConnectionReq, &[0, 1]);
            }
            _ => {}
        }
    }

    fn send_classic_signaling_packet(&mut self, cmd: SignalingCommand, data: &[u8]) {
        // create signaling packet
        let mut acl_buffer = [0 as u8; 200];
        self.create_classic_signaling_packet(&mut acl_buffer, cmd, data);
        self.request(&acl_buffer);
    }

    fn create_classic_signaling_packet(&mut self, acl_buffer: &mut [u8], cmd: SignalingCommand, option: &[u8]) {
        let mut len = 0;
        // clear data length field
        set_u16_le(&mut acl_buffer[2..4], len.clone());

        match cmd {
            SignalingCommand::CommandRejectRsp => {
                set_u16_le(&mut acl_buffer[4..6], SignalRejectReason::CommandNotUnderstood as u16);
                len += 2;
                // TODO: Reason Data
            }
            SignalingCommand::ConnectionReq => {
                set_u16_le(&mut acl_buffer[4..6], self.psm.clone());

                self.next_loacl_cid();
                set_u16_le(&mut acl_buffer[6..8], self.local_cid.clone());
                len += 4;
            }
            SignalingCommand::ConnectionRsp => {
                set_u16_le(&mut acl_buffer[4..6], self.remote_cid.clone());
                set_u16_le(&mut acl_buffer[6..8], self.local_cid.clone());

                // Result and Status send in option argument
            }
            SignalingCommand::ConfigurationReq => {
                set_u16_le(&mut acl_buffer[4..6], self.remote_cid.clone());
                let flags = 0x0000_u16;
                set_u16_le(&mut acl_buffer[6..8], flags);

                // Configuration Options send in option argument
            }
            SignalingCommand::ConfigurationRsp => {
                set_u16_le(&mut acl_buffer[4..6], self.local_cid.clone());
                let flags = 0x0000_u16;
                set_u16_le(&mut acl_buffer[6..8], flags);
                // TODO: Other result
                set_u16_le(&mut acl_buffer[8..10], SignalConfigurationResult::Successful as u16);
            }
            SignalingCommand::DisconnectionReq | SignalingCommand::DisconnectionRsp => {
                set_u16_le(&mut acl_buffer[4..6], self.remote_cid.clone());
                set_u16_le(&mut acl_buffer[6..8], self.local_cid.clone());
            }
            _ => {}
        }

        // octet 0: code
        acl_buffer[0] = cmd as u8;

        // octet 1: identifier
        self.next_sig_id();
        acl_buffer[1] = self.sig_seq_num.clone();

        // octet 2 and 3: data length
        len += (option.len() & 0xffff) as u16;
        set_u16_le(&mut acl_buffer[2..4], len);

        // octet..: option data
        // TODO
    }

    fn next_sig_id(&mut self) {
        if self.sig_seq_num == 0xff_u8 {
            self.sig_seq_num = 1;
        } else {
            self.sig_seq_num += 1;
        }
    }

    fn next_loacl_cid(&mut self) {
        if self.local_cid == 0 || self.local_cid == 0xffff_u16 {
            self.local_cid = 0x40;
        } else {
            self.local_cid += 1;
        }
    }
}

struct Signal {
    handle: u16,
    id: u8,
    code: u8,
    // date: u16,
}

fn set_u16_le(a: &mut [u8], v: u16) {
    a[0] = v as u8;
    a[1] = (v >> 8) as u8;
}

fn get_u16_le(a: &[u8]) -> u16 {
    a[0] as u16 + ((a[1] as u16) << (8 as u8)) as u16
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_create_signal_packet() {
        let mut channel = Channel::new(0);
        
        let mut acl_buffer = [0 as u8; 200];
        channel.create_classic_signaling_packet(&mut acl_buffer, SignalingCommand::ConnectionReq, &[]);
        let len = &acl_buffer[2..4];
        let len = get_u16_le(len) as usize + 4;
        // dbg!(&acl_buffer[0..len]);
        assert_eq!(&acl_buffer[0..len], [2, 1, 4, 0, 0, 0, 64, 0]);
    }
}
