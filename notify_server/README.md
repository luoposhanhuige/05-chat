
Here is the logic and architecture of the `notify_server` project:

### Purpose
The `notify_server` is a lightweight, asynchronous web service designed to push real-time notifications from the server to connected clients using **Server-Sent Events (SSE)**.

### Architecture
It is built using the `tokio` asynchronous runtime and the `axum` web framework. The architecture follows a standard separation of concerns:

1. **Entry Point (main.rs)**:
   * Initializes logging (`tracing`).
   * Binds a non-blocking TCP socket to `0.0.0.0:6687`.
   * Mounts the application router and starts the HTTP server.
2. **Routing Controller (lib.rs)**:
   * Defines the HTTP endpoints and wires them to their respective handlers.
   * `GET /`: Serves a static HTML page embedded directly into the binary via `include_str!`.
   * `GET /events`: Handles the real-time SSE stream connections.
3. **Core Logic (sse.rs)**:
   * Handles the unidirectional (server-to-client) streaming logic.
   * Extracts the client's `User-Agent` for logging.
   * Generates a continuous `Stream` of events.
4. **Client (index.html)**:
   * A basic frontend that uses the browser's native JavaScript `EventSource` API to connect to `/events` and listen for incoming messages.

### Execution Logic
1. A user opens their browser and navigates to `http://localhost:6687/`.
2. The server responds with index.html.
3. The JavaScript inside index.html executes `new EventSource("/events")`, which makes a long-lived HTTP GET request back to the server.
4. The Axum router routes this request to `sse_handler`.
5. `sse_handler` responds with an `Sse` wrapper. It creates an asynchronous stream that yields a new data payload (`"hi!"`) every 1 second.
6. The server keeps the HTTP connection open indefinitely, automatically sending keep-alive packets and streaming the "hi!" messages as they are generated.
7. The browser's console logs `"Got: hi!"` every second until the tab is closed.
