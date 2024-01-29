mod errors;
mod types;

#[cfg(test)]
mod test;

use std::collections::BTreeMap;
use netlink_packet_core::{NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_DUMP, NLM_F_REQUEST};
use netlink_packet_route::RouteNetlinkMessage;
use netlink_packet_route::tc::TcMessage;
use netlink_sys::constants::NETLINK_ROUTE;
use netlink_sys::{Socket, SocketAddr};

pub use errors::*;
pub use types::*;

pub type TcStats = BTreeMap<u32, Tc>;
pub type Result<T> = std::result::Result<T, TcError>;

/// Get list of all `tc` qdiscs and classes.
pub fn tc_stats() -> Result<TcStats> {
    read_tc_stats(&get_netlink_qdiscs)
}

fn read_tc_stats(netlink_retriever: &dyn Fn() -> Result<Vec<TcMessage>>) -> Result<TcStats> {
    let messages = netlink_retriever()?;
    let tc_stats = messages
        .into_iter()
        .map(|msg| Tc::new(&msg))
        .map(|tc| (tc.index, tc))
        .collect();

    Ok(tc_stats)
}

fn get_netlink_qdiscs() -> Result<Vec<TcMessage>> {
    // open a socket
    let socket = Socket::new(NETLINK_ROUTE).map_err(|e| TcError::Netlink(e.to_string()))?;
    socket.connect(&SocketAddr::new(0, 0)).map_err(|e| TcError::Netlink(e.to_string()))?;

    // create a netlink request
    let mut nl_hdr = NetlinkHeader::default();
    nl_hdr.flags = NLM_F_REQUEST | NLM_F_DUMP;
    let msg = RouteNetlinkMessage::GetQueueDiscipline(TcMessage::default());
    let mut packet = NetlinkMessage::new(nl_hdr, NetlinkPayload::from(msg));
    packet.finalize();
    let mut buf = vec![0; packet.header.length as usize];
    packet.serialize(&mut buf[..]);

    // if buf.len() != packet.buffer_len() {
    //     return TcError::Netlink(io::Error::new(
    //         io::ErrorKind::Other,
    //         "Failed to serialize packet",
    //     )
    // }

    // send the request
    socket.send(&buf[..], 0).map_err(|e| TcError::Netlink(e.to_string()))?;

    // receive the response
    let mut recv_buf = vec![0; 4096];
    let mut offset = 0;
    let mut response = Vec::new();
    'out: while let Ok(size) = socket.recv(&mut &mut recv_buf[offset..], 0) {
        loop {
            let bytes = &recv_buf[offset..];
            let rx_packet = <NetlinkMessage<RouteNetlinkMessage>>::deserialize(bytes)
                .map_err(|e| TcError::Netlink(e.to_string()))?;
            response.push(rx_packet.clone());
            let payload = rx_packet.payload;
            if let NetlinkPayload::Error(err) = payload {
                return Err(TcError::Netlink(err.to_string()));
            }
            if let NetlinkPayload::Done(_) = payload {
                break 'out;
            }

            offset += rx_packet.header.length as usize;
            if offset == size || rx_packet.header.length == 0 {
                offset = 0;
                break;
            }
        }
    }

    let mut tc_msgs = Vec::new();
    for msg in response {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewQueueDiscipline(tc)) = msg.payload {
            tc_msgs.push(tc);
        }
    }

    return Ok(tc_msgs);
}
