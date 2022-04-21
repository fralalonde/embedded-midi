//! USB-MIDI Event Packet definitions
//! USB-MIDI is a superset of the MIDI protocol

use crate::message::Message;
use core::convert::{TryFrom};
use crate::{MidiError, Channel, channel};
use crate::status::{Status, status_byte, SYSEX_START, SYSEX_END};
use CodeIndexNumber::*;

use num_enum::UnsafeFromPrimitive;

pub type CableNumber = u8;

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Packet {
    bytes: [u8; 4]
}

impl Packet {
    pub fn from_raw(bytes: [u8; 4]) -> Self {
        Packet { bytes }
    }

    pub fn cable_number(&self) -> CableNumber {
        self.bytes[0] >> 4
    }

    pub fn code_index_number(&self) -> CodeIndexNumber {
        CodeIndexNumber::from(self.bytes[0] & 0x0F)
    }

    pub fn status(&self) -> Option<Status> {
        let payload = self.payload();
        if payload.is_empty() {
            None
        } else {
            Status::try_from(self.payload()[0]).ok()
        }
    }

    pub fn channel(&self) -> Option<Channel> {
        let byte = self.bytes[1];
        if byte < NoteOff as u8 {
            None
        } else {
            Some(channel(byte))
        }
    }

    /// Payload
    pub fn payload(&self) -> &[u8] {
        let cin = self.code_index_number();
        &self.bytes[1..cin.payload_len() + 1]
    }

    pub fn with_cable_num(mut self, cable_number: CableNumber) -> Self {
        self.bytes[0] = self.bytes[0] & 0x0F | u8::from(cable_number) << 4;
        self
    }

    /// Sysex body _excludes_ SYSEX_START and SYSEX_END markers
    /// Return an empty slice if packet hold no sysex data
    pub fn sysex_body(&self) -> &[u8] {
        match self.code_index_number() {
            Sysex =>
                if self.bytes[1] == SYSEX_START {
                    &self.bytes[2..]
                } else {
                    &self.bytes[1..]
                }
            SysexEndsNext2 => &self.bytes[1..2],
            SysexEndsNext3 => &self.bytes[1..3],
            _ => &[]
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<Message> for Packet {
    fn from(message: Message) -> Self {
        let mut packet = [0; 4];
        packet[0] = CodeIndexNumber::from(message) as u8;
        if let Some(byte) = status_byte(&message) {
            packet[1] = byte;
        }
        match message {
            Message::NoteOff(_ch, note, vel) => {
                packet[2] = note as u8;
                packet[3] = u8::from(vel);
            }
            Message::NoteOn(_, note, vel) => {
                packet[2] = note as u8;
                packet[3] = u8::from(vel);
            }
            Message::NotePressure(_, note, pres) => {
                packet[2] = note as u8;
                packet[3] = u8::from(pres);
            }
            Message::ChannelPressure(_, pres) => {
                packet[2] = u8::from(pres);
            }
            Message::ProgramChange(_, patch) => {
                packet[2] = u8::from(patch);
            }
            Message::ControlChange(_, ctrl, val) => {
                packet[2] = u8::from(ctrl);
                packet[3] = u8::from(val);
            }
            Message::PitchBend(_, bend) => {
                let (lsb, msb) = bend.into();
                packet[2] = u8::from(lsb);
                packet[3] = u8::from(msb);
            }
            Message::TimeCodeQuarterFrame(val) => {
                packet[2] = u8::from(val);
            }
            Message::SongPositionPointer(p1, p2) => {
                packet[2] = u8::from(p1);
                packet[3] = u8::from(p2);
            }
            Message::SongSelect(song) => {
                packet[2] = u8::from(song);
            }

            // Sysex packets will probably not be generated from messages,
            // but let's support it for completeness
            Message::SysexBegin(b1, b2) => {
                packet[1] = SYSEX_START;
                packet[2] = b1;
                packet[3] = b2;
            }
            Message::SysexCont(b1, b2, b3) => {
                packet[1] = b1;
                packet[2] = b2;
                packet[3] = b3;
            }
            Message::SysexEnd => {
                packet[1] = SYSEX_END;
            }
            Message::SysexEnd1(b1) => {
                packet[1] = b1;
                packet[2] = SYSEX_END;
            }
            Message::SysexEnd2(b1, b2) => {
                packet[1] = b1;
                packet[2] = b2;
                packet[3] = SYSEX_END;
            }

            Message::SysexEmpty => {
                packet[1] = SYSEX_START;
                packet[2] = SYSEX_END;
            }
            Message::SysexSingleByte(b1) => {
                packet[1] = SYSEX_START;
                packet[2] = b1;
                packet[3] = SYSEX_END;
            }

            // remaining messages are single byte (status only)
            _ => {}
        }
        Self::from_raw(packet)
    }
}

/// The Code Index Number(CIN) indicates the classification
/// of the bytes in the MIDI_x fields
#[allow(unused)]
#[derive(Debug, Eq, PartialEq, UnsafeFromPrimitive)]
#[repr(u8)]
pub enum CodeIndexNumber {
    /// Miscellaneous function codes. Reserved for future extensions
    MiscFunction = 0x00,
    /// Cable events. Reserved for future expansion.
    CableEvents = 0x1,
    /// Two-byte System Common messages like MTC, SongSelect, etc.
    SystemCommonLen2 = 0x2,
    /// Three-byte System Common messages like SPP, etc.
    SystemCommonLen3 = 0x3,
    /// SysEx starts or continues
    Sysex = 0x4,
    /// Single-byte System Common Message or SysEx ends with following single byte.
    SystemCommonLen1 = 0x5,
    /// SysEx ends with following two bytes
    SysexEndsNext2 = 0x6,
    /// SysEx ends with following three bytes
    SysexEndsNext3 = 0x7,

    /// Note Off
    NoteOff = 0x8,
    /// Note On
    NoteOn = 0x9,
    /// Poly-KeyPess
    PolyKeypress = 0xA,
    /// Control Change
    ControlChange = 0xB,
    /// Program Change
    ProgramChange = 0xC,
    /// Channel Pressure
    ChannelPressure = 0xD,
    /// Pitch Bend Change
    PitchbendChange = 0xE,

    /// Single Byte
    SingleByte = 0xF,
}

impl From<u8> for CodeIndexNumber {
    fn from(byte: u8) -> Self {
        unsafe { CodeIndexNumber::from_unchecked(byte & 0x0F) }
    }
}

impl From<Status> for CodeIndexNumber {
    fn from(status: Status) -> Self {
        match status {
            Status::SysexStart => Sysex,
            Status::TimeCodeQuarterFrame => SystemCommonLen2,
            Status::SongPositionPointer => SystemCommonLen3,
            Status::TuneRequest => SystemCommonLen1,
            Status::SongSelect => SystemCommonLen2,
            Status::TimingClock => SystemCommonLen1,
            Status::MeasureEnd => SystemCommonLen2,
            Status::Start => SystemCommonLen1,
            Status::Continue => SystemCommonLen1,
            Status::Stop => SystemCommonLen1,
            Status::ActiveSensing => SystemCommonLen1,
            Status::SystemReset => SystemCommonLen1,

            channel_status => unsafe { CodeIndexNumber::from_unchecked(channel_status as u8 >> 4) },
        }
    }
}

impl From<Message> for CodeIndexNumber {
    fn from(message: Message) -> Self {
        match message {
            Message::NoteOff(_, _, _) => CodeIndexNumber::NoteOff,
            Message::NoteOn(_, _, _) => CodeIndexNumber::NoteOn,
            Message::NotePressure(_, _, _) => CodeIndexNumber::PolyKeypress,
            Message::ChannelPressure(_, _) => CodeIndexNumber::ChannelPressure,
            Message::ProgramChange(_, _) => CodeIndexNumber::ProgramChange,
            Message::ControlChange(_, _, _) => CodeIndexNumber::ControlChange,
            Message::PitchBend(_, _) => CodeIndexNumber::PitchbendChange,
            Message::TimeCodeQuarterFrame(_) => CodeIndexNumber::SystemCommonLen2,
            Message::SongPositionPointer(_, _) => CodeIndexNumber::SystemCommonLen3,
            Message::SongSelect(_) => CodeIndexNumber::SystemCommonLen2,
            Message::TuneRequest => CodeIndexNumber::SystemCommonLen1,
            Message::TimingClock => CodeIndexNumber::SystemCommonLen1,
            Message::MeasureEnd(_) => CodeIndexNumber::SystemCommonLen2,
            Message::Start => CodeIndexNumber::SystemCommonLen1,
            Message::Continue => CodeIndexNumber::SystemCommonLen1,
            Message::Stop => CodeIndexNumber::SystemCommonLen1,
            Message::ActiveSensing => CodeIndexNumber::SystemCommonLen1,
            Message::SystemReset => CodeIndexNumber::NoteOn,

            Message::SysexBegin(..) => CodeIndexNumber::Sysex,
            Message::SysexCont(..) => CodeIndexNumber::Sysex,
            Message::SysexEnd => CodeIndexNumber::SystemCommonLen1,
            Message::SysexEnd1(..) => CodeIndexNumber::SysexEndsNext2,
            Message::SysexEnd2(..) => CodeIndexNumber::SysexEndsNext3,
            Message::SysexEmpty => CodeIndexNumber::SysexEndsNext2,
            Message::SysexSingleByte(..) => CodeIndexNumber::SysexEndsNext3,
        }
    }
}

impl CodeIndexNumber {
    pub fn end_sysex(len: u8) -> Result<CodeIndexNumber, MidiError> {
        match len {
            1 => Ok(SystemCommonLen1),
            2 => Ok(SysexEndsNext2),
            3 => Ok(SysexEndsNext3),
            _ => Err(MidiError::SysexOutOfBounds)
        }
    }

    pub fn payload_len(&self) -> usize {
        match self {
            MiscFunction => 0,
            CableEvents => 0,
            SystemCommonLen2 => 2,
            SystemCommonLen3 => 3,
            Sysex => 3,
            SystemCommonLen1 => 1,
            SysexEndsNext2 => 2,
            SysexEndsNext3 => 3,
            NoteOff => 3,
            NoteOn => 3,
            PolyKeypress => 3,
            ControlChange => 3,
            ProgramChange => 2,
            ChannelPressure => 2,
            PitchbendChange => 3,
            SingleByte => 1,
        }
    }
}
