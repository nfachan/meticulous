mod dispatcher;
mod executor;

use crate::{channel_reader, proto, task, Error, ExecutionDetails, ExecutionId, Result};

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
    slots: u32,
    dispatcher_receiver: DispatcherReceiver,
    dispatcher_sender: DispatcherSender,
    broker_socket_sender: BrokerSocketSender,
) -> Result<()> {
    let adapter = DispatcherAdapter {
        dispatcher_sender,
        broker_socket_sender,
    };
    let mut dispatcher = dispatcher::Dispatcher::new(adapter, slots);
    channel_reader::run(dispatcher_receiver, move |msg| {
        dispatcher.receive_message(msg)
    })
    .await;
    Ok(())
}

async fn signal_handler(kind: tokio::signal::unix::SignalKind) -> Result<()> {
    tokio::signal::unix::signal(kind)?.recv().await;
    Err(Error::msg(format!("received signal {:?}", kind)))
}

/// The main function for the worker. This should be called on a task of its own. It will return
/// when a signal is received or when one of the worker tasks completes because of an error.
pub async fn main(name: String, slots: u32, broker_addr: std::net::SocketAddr) -> Result<()> {
    let (read_stream, mut write_stream) = tokio::net::TcpStream::connect(&broker_addr)
        .await?
        .into_split();
    let read_stream = tokio::io::BufReader::new(read_stream);

    proto::write_message(
        &mut write_stream,
        proto::Hello::Worker(proto::WorkerHello { name, slots }),
    )
    .await?;

    let (dispatcher_sender, dispatcher_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (broker_socket_sender, broker_socket_receiver) = tokio::sync::mpsc::unbounded_channel();

    let dispatcher_sender_clone = dispatcher_sender.clone();

    tokio::select! {
        res = task::spawn(async move {
            proto::socket_reader::<proto::WorkerRequest, dispatcher::Message>(read_stream, dispatcher_sender_clone)
                .await
        }) => res?,
        res = task::spawn(async move {
            proto::socket_writer(broker_socket_receiver, write_stream).await
        }) => res?,
        res = task::spawn(async move {
            dispatcher_main(slots, dispatcher_receiver, dispatcher_sender, broker_socket_sender).await
        }) => res?,
        res = task::spawn(async {
            signal_handler(tokio::signal::unix::SignalKind::interrupt()).await
        }) => res?,
        res = task::spawn(async {
            signal_handler(tokio::signal::unix::SignalKind::terminate()).await
        }) => res?,
    }
}