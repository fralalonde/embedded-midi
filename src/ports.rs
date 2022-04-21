use core::fmt::{Debug, Formatter};
use core::sync::atomic::AtomicU16;
use heapless::FnvIndexMap;
use crate::{MidiError, Packet, PacketList};

const MAX_PORTS: usize = 8;

pub type PortId = u16;

static NEXT_MIDI_PORT_ID: AtomicU16 = AtomicU16::new(1);

pub type MidiFn = Option<&'static mut (dyn FnMut(PacketList) + Send + Sync)>;

pub enum PortType {
    USB,
    Serial,
}

trait PortRegistry {
    fn add_port(&mut self, port_type: PortType) -> PortHandle;
    fn remove_port(&mut self, handle: PortHandle);
    fn set_input(&mut self, handle: PortHandle, port: MidiFn);
    fn set_output(&mut self, handle: PortHandle, port: MidiFn);
}

#[derive(Debug)]
struct MidiPortPair {
    input: MidiFn,
    output: MidiFn,
}

static mut MIDI_PORTS: FnvIndexMap<PortId, MidiPortPair, MAX_PORTS> = FnvIndexMap::new();

pub trait Receive {
    fn receive(&mut self) -> Result<Option<Packet>, MidiError>;
}

/// Set callback on reception of MIDI packets
pub trait ReceiveListener {
    fn on_receive(&mut self, listener: Option<&'static mut (dyn FnMut(PacketList) + Send + Sync)>);
}

#[derive(Debug)]
pub struct MidiWritePort {
    ep: SingleEp,
    buffer: ArrayQueue<Packet, 17>,
}

pub struct MidiReadPort {
    cb: SpinMutex<Option<MidiFn>>,
}

impl Debug for MidiReadPort {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.ep.fmt(f)
    }
}

impl ReceiveListener for MidiReadPort {
    fn on_receive(&mut self, listener: Option<&'static mut (dyn FnMut(PacketList) + Send + Sync)>) {
        *self.cb.lock() = listener
    }
}
