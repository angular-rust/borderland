# borderland

Application platform powers Loadbalancers, Microservices and API Gateways

This project was started from ugly fun [example](ttps://github.com/PritiKumr/rust-httpd). But this project is not a simple HTTP load balncer. We dont need it. We need to balance our application payload. So let we use somthing like a application bus to communicate with application logic.

Also there standard practice to use external crates(modules,libraries). But lot of them suffer from overprogramming, not clear implementation of specifications and unnecessary extra functionality. So i liked the starter point of project and i would like to use less external code in that project. Because "less code" is mean less errors and faster executon, which is main goal.

## How to run?

Make sure you have `cargo` [installed](https://www.rust-lang.org/en-US/install.html) and run the command `cargo run` to boot up the server.

Visit `locahost:8080` from your browser.

### What can it do now?

1.  **Say hello world** - visit `localhost:8888/api/v1`

2.  **Serve static files** - visit `localhost:8888/files/index.html` - this will serve the `index.html` file from the `www` folder in the repo root. Place any other file inside `www` and they can be served similarly (using the `/files` prefix - this will be configurable by the user in future, just like in Apache and Nginx).

## Roadmap

- [x] HTTP Scheme redirect to HTTPS by design
- [ ] Routing
- [ ] Session handling
- [ ] Datastorage
- [ ] Application Bus
- [ ] Internal application logic

## Design notes

### Routing

First look at design.

```rust
    let router = Rourer::new()
        .get(RegexMatch{"/article/(\d{4})"}, articlesHandler)
        .post(StrongMatch{"/api/v1"}, apiHandler)
        .get(StrongMatch{"/"}, landingHandler)
```

### Sessions

Session is the data associated with user interaction. Session ID - agnostic way to associate user with session, not matter to use cookies or somthing else.

### Datastorage

In project perspective it should be key - value storage to replace old solutions like Memcached or Redis. Why we need own datastorage? Coz the old-fashion solutions is designed for common usage few years ago and at that moment there was absoluttelly different technologies, tools and goals than exists now. So it's not appropriate to use old-school tools at this time. We need something more productive.

## Application bus

I dont think the usage of HTTP protocol between frontend server and application logic is productive. There exists double work to parse HTTP - one time on the frontend server, other time on the applicaation logic side. So we need more productive way. Also we need to be scalable, coz we can divide the powerload between nodes. What i mean in term "powerload". Powerload is not only application logic requests but the data requests. So the application bus it is a trasport for application reqests and data requests. There no need to use database drivers on application logic side, coz we can use the same transport for data.
As i said "less code is more productive and stable".

## Application Transport

We have a two transport bus. One is persistent connection, other is not. We should to implement transport to deliver messages using second way. The task is to deliver few messages in a request. It is possible when we'll use the multiplexing. We will inject it between HTTP (for example) and application protocol. When we do it the requewst based protocol will look like a persistent, with "defer" feature of course. We do not guaranty that the message will deliver at the time, but it will send and probably delivered in the session.

## Security

All interaction with the server should be secure, no matter whether the request to the server has come from outside, or the application's logic handler interacts with the server or we try change the server configuration. All things should be secure.
