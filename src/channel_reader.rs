//! Read messages from a channel.

/// Read messages from a channel, calling an individual function on each one. Return when there are
/// no more channel senders.
pub async fn run<MessageT>(
    mut channel: tokio::sync::mpsc::UnboundedReceiver<MessageT>,
    mut processor: impl FnMut(MessageT),
) {
    while let Some(x) = channel.recv().await {
        processor(x);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn no_messages() {
        let (_, rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
        let mut vec = vec![];
        run(rx, |s| vec.push(s)).await;
        assert_eq!(vec, vec![]);
    }

    #[tokio::test]
    async fn one_messages() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn(async move { tx.send(1).unwrap() });
        let mut vec = vec![];
        run(rx, |s| vec.push(s)).await;

        assert_eq!(vec, vec![1]);
    }

    #[tokio::test]
    async fn three_messages() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn(async move {
            tx.send(1).unwrap();
            tx.send(2).unwrap();
            tx.send(3).unwrap();
        });
        let mut vec = vec![];
        run(rx, |s| vec.push(s)).await;

        assert_eq!(vec, vec![1, 2, 3]);
    }
}
