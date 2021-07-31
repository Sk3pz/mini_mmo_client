use std::net::TcpStream;
use crate::packet_capnp::event;
use capnp::message::{TypedReader, Builder, HeapAllocator};
use capnp::serialize;
use crate::utils::systime;
use crate::network::message::Message;

/// Sends a message event
pub fn write_event_message<S: Into<String>>(mut stream: &TcpStream, msg_str: S, data: S) -> ::capnp::Result<()> {
    let mut message = Builder::new_default();
    {
        let mut ev = message.init_root::<event::Builder>();
        ev.set_disconnect(false);
        let mut msgdata = ev.init_message();
        msgdata.set_message(msg_str.into().as_str());
        msgdata.set_data(data.into().as_str());
    }
    serialize::write_message(&mut stream, &message)
}

/// Sends a keepalive request or response
pub fn write_event_keepalive(mut stream: &TcpStream) -> ::capnp::Result<()> {
    let mut message = Builder::new_default();
    {
        let mut ev = message.init_root::<event::Builder>();
        ev.set_disconnect(false);
        ev.set_keepalive(systime().as_secs());
    }
    serialize::write_message(&mut stream, &message)
}

/// Sends an error
pub fn write_event_error<S: Into<String>>(mut stream: &TcpStream, error: S, disconnect: bool) -> ::capnp::Result<()> {
    let mut message = Builder::new_default();
    {
        let mut ev = message.init_root::<event::Builder>();
        ev.set_disconnect(disconnect);
        ev.set_error(error.into().as_str());
    }
    serialize::write_message(&mut stream, &message)
}

/// Reads an event packet, and returns it's data
/// Returns message, keepalive_time, an error, and a disconnect flag
pub fn read_event(mut stream: &TcpStream) -> (Option<Message>, Option<u64>, Option<String>, bool) {
    let mut needs_to_disconnect = false;

    // read the event
    let message_reader_result = serialize::read_message(&mut stream, ::capnp::message::ReaderOptions::new());
    if message_reader_result.is_err() { // disconnected
        return (None, None, None, true);
    }
    let message_reader = message_reader_result.unwrap();
    // store the event in a Reader to obtain data out of it
    let ev_raw = message_reader.get_root::<event::Reader>();
    if ev_raw.is_err() {
        return (None, None, None, true);
    }
    let ev = ev_raw.unwrap();

    needs_to_disconnect = ev.get_disconnect();

    // the event is a Cap'n Proto Union, so go through which type of event it is
    return match ev.which() {
        Ok(event::Message(msg)) => {
            let raw_msg = msg.unwrap();
            let m = Message {
                message: raw_msg.get_message().unwrap().to_string(),
                data: raw_msg.get_data().unwrap().to_string(),
            };
            (Some(m), None, None, needs_to_disconnect)
        }
        Ok(event::Keepalive(st)) => {
            (None, Some(st), None, ev.get_disconnect())
        }
        Ok(event::Error(err)) => {
            (None, None, Some(err.unwrap().to_string()), needs_to_disconnect)
        }
        Err(::capnp::NotInSchema(_)) => {
            // todo: error?
            (None, None, Some(String::from("Invalid EntryPoint - no version or login data found!")), needs_to_disconnect)
        }
    }
}