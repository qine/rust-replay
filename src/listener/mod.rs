use pnet::datalink::{self, NetworkInterface};

use pnet::packet::Packet;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::udp::UdpPacket;
use std::env;
use std::io::{self, Write};
use std::process;
use std::net::IpAddr;
use pnet::datalink::Channel::Ethernet;

use std::time::Duration;

use dumper;

pub struct Listener {
    dumper: dumper::Dumper
}

impl Listener {
    pub fn new() -> Listener {
        Listener {
            dumper: dumper::Dumper::new(),
        }
    }

    pub fn run(&mut self, network_interface: &NetworkInterface) {
        let (_, mut rx) = match datalink::channel(network_interface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("packetdump: unhandled channel type: {}"),
            Err(e) => panic!("packetdump: unable to create channel: {}", e),
        };

        let mut iter = rx.iter();
        loop {
            match iter.next() {
                Ok(packet) => {
                    self.handle_packet(&network_interface.name, &packet);
                },
                Err(e) => println!("packetdump: unable to receive packet: {}", e)
            }
        }
    }

    fn handle_udp_packet(&mut self, interface_name: &str, source: IpAddr, destination: IpAddr, packet: &[u8]) {
        let udp = UdpPacket::new(packet);

        if let Some(udp) = udp {
            self.dumper.dump(packet);

            println!("[{}]: UDP Packet: {}:{} > {}:{}; length: {};",
                     interface_name,
                     source,
                     udp.get_source(),
                     destination,
                     udp.get_destination(),
                     udp.get_length());

            //            self.dumper.dump(packet);
        } else {
            println!("[{}]: Malformed UDP Packet", interface_name);
        }
    }

    fn handle_transport_protocol(&mut self, interface_name: &str,
                                 source: IpAddr,
                                 destination: IpAddr,
                                 protocol: IpNextHeaderProtocol,
                                 packet: &[u8]) {
        match protocol {
            IpNextHeaderProtocols::Udp => {
                self.handle_udp_packet(interface_name, source, destination, packet)
            }
            _ => {
                println!("[{}]: Unknown {} packet: {} > {}; protocol: {:?} length: {}",
                         interface_name,
                         match source {
                             IpAddr::V4(..) => "IPv4",
                             _ => "IPv6",
                         },
                         source,
                         destination,
                         protocol,
                         packet.len())
            }
        }
    }

    fn handle_ipv4_packet(&mut self, interface_name: &str, ethernet: &EthernetPacket) {
        let header = Ipv4Packet::new(ethernet.payload());
        if let Some(header) = header {
            self.handle_transport_protocol(interface_name,
                                           IpAddr::V4(header.get_source()),
                                           IpAddr::V4(header.get_destination()),
                                           header.get_next_level_protocol(),
                                           header.payload());
        } else {
            println!("[{}]: Malformed IPv4 Packet", interface_name);
        }
    }

    fn handle_ipv6_packet(&mut self, interface_name: &str, ethernet: &EthernetPacket) {
        let header = Ipv6Packet::new(ethernet.payload());
        if let Some(header) = header {
            self.handle_transport_protocol(interface_name,
                                           IpAddr::V6(header.get_source()),
                                           IpAddr::V6(header.get_destination()),
                                           header.get_next_header(),
                                           header.payload());
        } else {
            println!("[{}]: Malformed IPv6 Packet", interface_name);
        }
    }

    fn handle_packet(&mut self, interface_name: &str, ethernet: &EthernetPacket) {
        match ethernet.get_ethertype() {
            EtherTypes::Ipv4 => self.handle_ipv4_packet(interface_name, ethernet),
            EtherTypes::Ipv6 => self.handle_ipv6_packet(interface_name, ethernet),
            _ => {
                println!("[{}]: Unknown packet: {} > {}; ethertype: {:?} length: {}",
                         interface_name,
                         ethernet.get_source(),
                         ethernet.get_destination(),
                         ethernet.get_ethertype(),
                         ethernet.packet().len())
            }
        }
    }
}