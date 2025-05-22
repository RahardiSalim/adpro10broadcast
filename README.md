## Experiment 1.1.

This experiment demonstrates how asynchronous programming is effectively used in a real-time broadcast chat application using WebSockets in Rust. By running a server and multiple clients, we can observe how messages typed by one client are instantly received by all others, showcasing non-blocking communication. Each client handles sending and receiving concurrently using `tokio::spawn` and `tokio::select!`, ensuring smooth and responsive interaction. The server utilizes a broadcast channel to relay messages to all connected clients efficiently. This setup highlights the suitability of asynchronous programming for scenarios involving multiple simultaneous I/O operations, such as chat apps or live feeds.

![with drop](asset/original.png)

## Experiment 1.2.

In this experiment, we changed the WebSocket port from the default `2000` to `8080` to better simulate how ports can be adjusted based on deployment environments. Since WebSocket communication involves both a server and a client, we had to update the port in two places. On the server side, the line `TcpListener::bind("127.0.0.1:2000")` was modified to `127.0.0.1:8080`, and on the client side, the WebSocket URI `ws://127.0.0.1:2000` was updated to `ws://127.0.0.1:8080`. This change ensures both components are trying to connect on the same port using the same WebSocket protocol. After the update, the chat application still functioned correctly, with clients able to send and receive messages just as before, confirming that the port change was successful.

![with drop](asset/8000.png)

## Experiment 1.3.

For this experiment, we modified the server so that every message sent includes the sender's IP address and port. This change was made by appending the sender's `SocketAddr` to each message before it is broadcast to all clients. The purpose is to allow clients to identify where each message is coming from, even though we haven’t implemented a user naming system yet. As a result, every message received by a client will be displayed in a format like “From 127.0.0.1:49123: hello”, making it easier to recognize the sender. This modification helps us understand how additional information can be embedded in asynchronous communication using WebSocket.


![with drop](asset/small_change.png)
