# self_msg_broker

A rewritten version of the golang message broker in rust
https://github.com/MeDaLL1ST/MessageBroker

## Subscribing description

To subscribe to the key, you need to create a websocket connection to the endpoint /subscribe, and pass the "Authorization" field containing the authorization key to headers. After the connection is established, you need to send JSON with a single key object: {"key":"some_key"}. The connection will subscribe to the key some_key, and you can re-subscribe to another key without disconnecting the connection by simply sending the JSON again.

## Subscribing adding

To send the key to the broker, you need to send an http request with JSON of the following type to the endpoint /add: {"key":"some_key","value":"some_value'"}. Authorization token is the same as for /subscribe.

## ENV

Do not forget to create a .env file in the root of the application with the following contents:
API_KEY=some_key
PORT=http_port
MAX_THREADS=max_threads

## Load balancing

The message broker can also perform a load balancing function. If you make 3 connections and subscribe to the same key, then when you send the value, it will pass to everyone in order. If there was no subscription to the sent key, it is lost.

## Information

By endpoints /list and /info you can see all keys for which subscriptions currently exist, and information about the number of subscribers to the key when transmitted in the body of a json request of the form: {"key":"somekey"}.

## Running

    docker build -t image .
    docker run -d -p 8080:8080 --restart=always image
