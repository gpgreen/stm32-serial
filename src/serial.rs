use serial_packet_parser::{PacketParser, USARTPacket};

const BAD_CHECKSUM: u8 = 253;
const UNKNOWN_ADDRESS: u8 = 254;
const INVALID_BATCH_SIZE: u8 = 255;

// constants for address range sizes
const CONFIG_ARRAY_SIZE: u8 = 64;
const DATA_ARRAY_SIZE: u8 = 60;
const COMMAND_COUNT: u8 = 12;

// constants for address starts
const DATA_ARRAY_START: u8 = 80;
const COMMAND_ADDR_START: u8 = 160;

//assert! (CONFIG_ARRAY_SIZE < DATA_ARRAY_START);

pub enum USARTPacketType {
    Config,
    Data,
    Command,
    Unknown,
}

pub fn usartpacket_type(pkt: &USARTPacket) -> USARTPacketType {
    if pkt.address < CONFIG_ARRAY_SIZE {
        USARTPacketType::Config
    } else if pkt.address >= DATA_ARRAY_START && pkt.address < DATA_ARRAY_START + DATA_ARRAY_SIZE {
        USARTPacketType::Data
    } else if pkt.address >= COMMAND_ADDR_START && pkt.address < COMMAND_ADDR_START + COMMAND_COUNT
    {
        USARTPacketType::Command
    } else {
        USARTPacketType::Unknown
    }
}

pub struct SerialHandler {
    pkt_buf: USARTPacket,
    serial_rx_queue: [u8; 1024],
    serial_tx_queue: [u8; 512],
}

impl SerialHandler {
    pub fn new() -> SerialHandler {
        SerialHandler {
            pkt_buf: USARTPacket::new(),
            serial_rx_queue: [0; 1024],
            serial_tx_queue: [0; 512],
        }
    }

    pub fn receive_byte(&mut self, byte: u8, parser: PacketParser) -> PacketParser {
        let mut p = parser.parse_received_byte(byte, &mut self.pkt_buf);
        match p.have_complete_packet() {
            Some(_) => {
                match self.pkt_buf.compare_checksum() {
                    true => match usartpacket_type(&self.pkt_buf) {
                        USARTPacketType::Unknown => self.queue_tx_unknown_address_packet(),
                        _ => self.queue_rx_packet(),
                    },
                    false => self.queue_tx_bad_checksum_packet(),
                };
                p = PacketParser::new();
            }
            None => {}
        };
        p
    }

    fn queue_rx_packet(&mut self) {
        self.serial_rx_queue[0] = self.pkt_buf.pt;
        self.serial_rx_queue[1] = self.pkt_buf.address;
        self.serial_rx_queue[2] = self.pkt_buf.datalen;
        let mut i: usize = 0;
        if self.pkt_buf.datalen > 0 {
            let len: usize = self.pkt_buf.datalen.into();
            while i < len {
                self.serial_rx_queue[i + 3] = self.pkt_buf.data[i];
                i = i + 1;
            }
        }
    }

    fn queue_tx_bad_checksum_packet(&mut self) {
        let pkt = USARTPacket {
            pt: 0b0000_0000,
            address: BAD_CHECKSUM,
            checksum: 0,
            datalen: 0,
            data: [0; 64],
        };
        let bytes = pkt.compute_checksum().to_be_bytes();
        self.serial_tx_queue[0] = 0b0000_0000;
        self.serial_tx_queue[1] = BAD_CHECKSUM;
        self.serial_tx_queue[2] = 0;
        self.serial_tx_queue[3] = bytes[0];
        self.serial_tx_queue[4] = bytes[1];
    }

    fn queue_tx_unknown_address_packet(&mut self) {
        let pkt = USARTPacket {
            pt: 0b0000_0000,
            address: UNKNOWN_ADDRESS,
            checksum: 0,
            datalen: 0,
            data: [0; 64],
        };
        let bytes = pkt.compute_checksum().to_be_bytes();
        self.serial_tx_queue[0] = 0b0000_0000;
        self.serial_tx_queue[1] = BAD_CHECKSUM;
        self.serial_tx_queue[2] = 0;
        self.serial_tx_queue[3] = bytes[0];
        self.serial_tx_queue[4] = bytes[1];
    }
}
