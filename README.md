# borderland
Application platform powers Loadbalancers, Microservices and API Gateways

## A WebServer in Rust for fun & learning

Just to see what it takes to build a HTTP web server and to learn Rust along the way.

### How to run?

Make sure you have `cargo` [installed](https://www.rust-lang.org/en-US/install.html) and run the command `cargo run` to boot up the server.

Visit `locahost:8888` from your browser.

### What can it do now?

1.  **Say hello world** - visit `localhost:8888/hello`

2.  **Serve static files** - visit `localhost:8888/files/index.html` - this will serve the `index.html` file from the `www` folder in the repo root. Place any other file inside `www` and they can be served similarly (using the `/files` prefix - this will be configurable by the user in future, just like in Apache and Nginx).

3.  **Execute CGI Scripts** - place any script inside the `cgi` folder and they can be executed by visiting `localhost:8888/cgi/script`. This is a very simplistic implementation. Planning to support `SCGI`. Maybe FastCGI in future.

### TODO

- Reverse Proxy

### Follow the project

We ([Steve](https://github.com/steverob) & [Preethi](https://github.com/PritiKumr)) will be posting updates about the project and will try to write stuff on Rust as we learn more about the language at our blog - [Adventures in Rust](https://medium.com/adventures-in-rust).

Started from ugly fun [example](ttps://github.com/PritiKumr/rust-httpd)

# Moving to TCPStream, Bye Tokio!

![](https://cdn-images-1.medium.com/freeze/max/30/1*OGxkKcM289IIglpQ6TMwdA.jpeg?q=20)

![](https://cdn-images-1.medium.com/max/1000/1*OGxkKcM289IIglpQ6TMwdA.jpeg)

![](https://cdn-images-1.medium.com/max/1000/1*OGxkKcM289IIglpQ6TMwdA.jpeg)

Hey people, the last time we wrote about our little web server project, we had build a simple static file server.

And you know, we were absolute newbies in Rust and were trying to get a hold of the ecosystem and the tooling while putting this together. Based on some quick research we found this promising networking library, [**_Tokio_**](https://tokio.rs/)**_._** And we thought we could use it as a base for our project.

As Tokio’s website puts it, Tokio is —

> A platform for writing fast networking code with Rust.

It sure sounded like something we’d want to use for our learning project. We dug through the guides which were basically delivered using few example apps. The first example they introduced us to, [writing an echo server with Tokio](https://tokio.rs/docs/getting-started/simple-server/), really showed how Tokio would allow us to cleanly implement a network stack complete from handing requests and responses to the protocol that’s being employed as well.

We were very excited and managed to hack together a simple static file server. But then, at one point our productivity started dropping. We hit the brakes and thought about our decision to go with Tokio and finally decided to drop the idea and go with something else. Before we tell you about the replacement, here’s our pain points. Hopefully we can learn a lesson or two from this.

1.  **_Hard time putting the abstractions to use — _**Since we were very new to Rust, we had a very hard time understanding the example code and reading through the docs and figuring out how to make Tokio’s abstractions work for us and where the boundaries lied. We basically ended up spending more time learning about these abstractions _(Codec, Protocol, Service, etc)_ than building useful things over them.
2.  **_Navigating the samples and projects that use Tokio — _**Since we were absolutely new to Rust, navigating even a simple piece of code was a difficult endeavour. And when we tried to learn from a some projects on GitHub that used Tokio to build stuff, we ended up jumping back and forth between the code and the docs a little too often than we’d like and even though we eventually ended up liking the docs, they were not very intuitive in the beginning. Tokio required us to understand a little too much _(io, core, proto, service, futures, etc..)_ and it ended up becoming overwhelming for us, fledgling Rust developers :)

To put it short, even after a simple static file server that we referred in our [first post](https://medium.com/adventures-in-rust/peek-a-boo-rust-cc46dee79ae4), we still did not have a clear understanding of how things were wired together right from socket creation to how the response was being sent out. And this was a big problem for us as the whole point of this project is to help us learn the very basics of building a server.

I’m sure if we come back to this post a few months down the line, we’re going to be bashing our amateur-selves for dropping Tokio for these reasons ;)

**Enter Rust’s own std::net module**

With Tokio gone, we looked toward Rust’s std::net module. After going through the docs and few examples, we felt this is what we should have used in the first place.

Just take a look at this code —

![](https://i.embed.ly/1/display/resize?url=https%3A%2F%2Favatars2.githubusercontent.com%2Fu%2F1220480%3Fv%3D3%26s%3D400&key=4fce0568f2ce49e8b54624ef71a8a5bd&width=40)

We had to spend a fraction of time that we spent with Tokio in order to understand this and put these modules to use.

Here’s what we did next —

1.  We setup a basic server that simply responds with _Hello World._ Not very useful indeed, but always a good start.
2.  Planned out the initial features that we want our web server to provide
    _1\. Static File Serving
    2\. Support CGI Scripting
    3\. Reverse Proxying_
3.  Integrated a [HTTP Parser crate](https://github.com/seanmonstar/httparse).
4.  Setup a very basic, temporary, hard-coded router.
5.  Completed the static file server. (We’re [not able to serve PDFs](https://github.com/PritiKumr/rust-httpd/issues/3) properly for some reason.)
6.  Concurrency by spawning new threads for incoming requests.
7.  500 error pages when there are errors ❤ ❤

Even if we could have made this progress just with Tokio, I don’t think we would have felt very confident with our code. Using std::net gave us just the bare minimum letting us build things on top.

We did a quick micro-benchmark and we got a throughput of almost 10000 requests per second, which is not that surprising :D Anyway, that’s a useless number at this point.

So yeah, that’s where we are at the moment. Our next step is to enable CGI Scripting. I am looking forward to that very much as it lets us do some really cool stuff.

Currently all our code lives in the `main.rs` file and hopefully as we make more progress we can break down and modularise the code better.

Incase, you want to see how our project is turning out, here’s a secret link to our [**Github repo**](https://github.com/PritiKumr/rust-httpd).

**P.S**  
If you’re looking out for my Partner-in-Science, [Steve Robinson](https://medium.com/@steverob) is him!
