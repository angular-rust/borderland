# borderland

Application platform powers Loadbalancers, Microservices and API Gateways

<p align="center">
    <img src="https://raw.githubusercontent.com/wiki/ohyo-io/borderland/images/logo_borderland.png" alt="Borderland logo" width="256" />
</p>

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
- [x] Moved to MIO
- [ ] Routing
- [ ] Session handling
- [ ] Datastorage
- [ ] Application Bus
- [ ] Internal application logic

## Changelog

- Started profiling coz move to use MIO doesnt give good results
