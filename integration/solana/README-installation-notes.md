# Installation Notes
if you are a first-time user of Docker or Rust, the notes below may help you to install some of the dependencies on a Mac or Linux workstation.

### Rust
We suggest that you install Rust using the 'rustup' tool. Rustup will install
the latest version of Rust, Cargo, and the other binaries used in Solana.

Follow the instructions at [Installing Rust](https://www.rust-lang.org/tools/install).

For Mac users, Homebrew is also an option.  The Mac Homebrew command is `brew install rustup` and then
`rustup-init`. See [Mac Setup](https://sourabhbajaj.com/mac-setup/Rust/) &
[Installing Rust](https://www.rust-lang.org/tools/install) for more details.

After installation, you should have `rustc`, `cargo`, & `rustup`. You should
also have `~/.cargo/bin` in your PATH environment variable.

### NodeJS/NPM
Fetch the `npm` dependencies, including `@solana/web3.js`, by running:
```bash
$ npm install
```

### Docker
Docker runs as a service and it needs to be running before you can start the
Solana cluster. The exact start method depends on your system and how you
installed docker.

#### Install and Start Docker On Linux
The instructions to install Docker have changed over time. If you have
previously installed Docker, this will be a good time to update your system.
See [Install Docker Engine on Ubuntu](https://docs.docker.com/engine/install/ubuntu/) for a step-by-step walk-through. When complete, `sudo docker run hello-world` should confirm that everything works correctly.

To run Docker without typing `sudo` every time, take a look at Step 2 of [How To Install and Use Docker on Ubuntu 18.04](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-docker-on-ubuntu-18-04)

#### Install and Start Docker On A Mac
Docker provides a desktop application for Mac at [Docker Desktop for Mac](https://hub.docker.com/editions/community/docker-ce-desktop-mac/) with additional instructions here [Install Docker Desktop on Mac](https://docs.docker.com/docker-for-mac/install/). If you install the Docker Desktop app, you can skip the HomeBrew instructions below. If `docker run hello-world` works, you are ready to Start the local Solana cluster.

If you are using HomeBrew on a Mac, the commands are:

```bash
$ brew install docker
$ brew install docker-machine
# The next two commands to install virtualbox & create a machine may need a
# password. You may also need to address a System Preference setting and
# re-try the installation.
$ brew cask install virtualbox
$ docker-machine create --driver virtualbox default
# To see config info:
$ docker-machine env default
# Port forwarding of 8899 from your OS to the Docker machine:
$ docker-machine ssh default -f -N -L 8899:localhost:8899
# To configure your shell to use the docker-machine
$ eval "$(docker-machine env default)"
```

NOTE: Later, you can run `docker-machine stop default` to stop the docker machine.

Resources for Mac HomeBrew users:
- https://medium.com/@yutafujii_59175/a-complete-one-by-one-guide-to-install-docker-on-your-mac-os-using-homebrew-e818eb4cfc3
- https://stackoverflow.com/questions/32174560/port-forwarding-in-docker-machine

### Git Repository
Clone the 'example-helloworld' repository into your development machine:
```bash
$ cd /path/to/your/work/folder/
$ git clone https://github.com/solana-labs/example-helloworld.git
$ cd example-helloworld
```
(If you plan to submit changes in a pull request, be sure to create a fork
first and then clone your fork.)
