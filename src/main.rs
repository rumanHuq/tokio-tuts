use bytes::Bytes;
use mini_redis::{self, client};

use tokio::{
	spawn,
	sync::{mpsc, oneshot},
};

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;
#[derive(Debug)]
enum Command {
	Get {
		key: String,
		resp: Responder<Option<Bytes>>,
	},
	Set {
		key: String,
		val: Bytes,
		resp: Responder<()>,
	},
}

#[tokio::main]
async fn main() {
	let (tx, mut rx) = mpsc::channel(32);
	let tx2 = tx.clone();

	let t1 = spawn(async move {
		let (resp_tx, resp_rx) = oneshot::channel();
		let cmd = Command::Get {
			key: "hello".to_string(),
			resp: resp_tx,
		};

		tx.send(cmd).await.unwrap();
		let res = resp_rx.await;
		println!("GOT => {:?}", res);
	});
	let t2 = spawn(async move {
		let (resp_tx, resp_rx) = oneshot::channel();
		let cmd = Command::Set {
			key: "foo".to_string(),
			val: "bar".into(),
			resp: resp_tx,
		};

		tx2.send(cmd).await.unwrap();
		let res = resp_rx.await;
		println!("GOT => {:?}", res);
	});

	let manager = spawn(async move {
		let mut client = client::connect("127.0.0.1:6379").await.unwrap();
		use Command::*;
		while let Some(cmd) = rx.recv().await {
			match cmd {
				Get { key, resp } => {
					let res = client.get(&key).await;
					let _ = resp.send(res);
				}
				Set { key, val, resp } => {
					let res = client.set(&key, val.into()).await;
					let _ = resp.send(res);
				}
			}
		}
	});

	t1.await.unwrap();
	t2.await.unwrap();
	manager.await.unwrap();
}
