//! Code for the worker binary.

pub mod cache;
mod dispatcher;
mod executor;

use crate::{channel_reader, proto, Error, ExecutionDetails, ExecutionId, Result};

type DispatcherReceiver = tokio::sync::mpsc::UnboundedReceiver<dispatcher::Message>;
type DispatcherSender = tokio::sync::mpsc::UnboundedSender<dispatcher::Message>;
type BrokerSocketSender = tokio::sync::mpsc::UnboundedSender<proto::WorkerResponse>;

struct DispatcherAdapter {
    dispatcher_sender: DispatcherSender,
    broker_socket_sender: BrokerSocketSender,
}

impl dispatcher::DispatcherDeps for DispatcherAdapter {
    type ExecutionHandle = executor::Handle;

    fn start_execution(
        &mut self,
        id: ExecutionId,
        details: ExecutionDetails,
    ) -> Self::ExecutionHandle {
        let sender = self.dispatcher_sender.clone();
        executor::start(details, move |result| {
            sender
                .send(dispatcher::Message::FromExecutor(id, result))
                .ok();
        })
    }

    fn send_response_to_broker(&mut self, message: proto::WorkerResponse) {
        self.broker_socket_sender.send(message).ok();
    }
}

async fn dispatcher_main(
    slots: usize,
    dispatcher_receiver: DispatcherReceiver,
    dispatcher_sender: DispatcherSender,
    broker_socket_sender: BrokerSocketSender,
) {
    let adapter = DispatcherAdapter {
        dispatcher_sender,
        broker_socket_sender,
    };
    let mut dispatcher = dispatcher::Dispatcher::new(adapter, slots);
    channel_reader::run(dispatcher_receiver, |msg| dispatcher.receive_message(msg)).await;
}

async fn signal_handler(kind: tokio::signal::unix::SignalKind) -> Result<()> {
    tokio::signal::unix::signal(kind)?.recv().await;
    Err(Error::msg(format!("received signal {kind:?}")))
}

/// The main function for the worker. This should be called on a task of its own. It will return
/// when a signal is received or when one of the worker tasks completes because of an error.
pub async fn main(name: String, slots: usize, broker_addr: std::net::SocketAddr) -> Result<()> {
    let (read_stream, mut write_stream) = tokio::net::TcpStream::connect(&broker_addr)
        .await?
        .into_split();
    let read_stream = tokio::io::BufReader::new(read_stream);

    proto::write_message(
        &mut write_stream,
        proto::Hello::Worker {
            name,
            slots: slots as u32,
        },
    )
    .await?;

    let (dispatcher_sender, dispatcher_receiver) = tokio::sync::mpsc::unbounded_channel();
    let dispatcher_sender_clone = dispatcher_sender.clone();

    let (broker_socket_sender, broker_socket_receiver) = tokio::sync::mpsc::unbounded_channel();

    let mut join_set = tokio::task::JoinSet::new();
    join_set.spawn(proto::socket_reader(
        read_stream,
        dispatcher_sender_clone,
        dispatcher::Message::FromBroker,
    ));
    join_set.spawn(proto::socket_writer(broker_socket_receiver, write_stream));
    join_set.spawn(async move {
        dispatcher_main(
            slots,
            dispatcher_receiver,
            dispatcher_sender,
            broker_socket_sender,
        )
        .await;
        Ok(())
    });
    join_set.spawn(signal_handler(tokio::signal::unix::SignalKind::interrupt()));
    join_set.spawn(signal_handler(tokio::signal::unix::SignalKind::terminate()));

    loop {
        join_set
            .join_next()
            .await
            .expect("at least one task should return an error")
            .expect("no task should panic or be canceled")?;
    }
}
