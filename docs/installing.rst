Installing Solang
=================

The Solang compiler is a single binary. It can be installed in many different
ways, so the method that suits your needs.

Download release binaries
-------------------------

For Ubuntu, there is an x86-64 binary available in the github releases:

`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.3/solang_linux>`_

For Windows x64, there is a pre-built binary available here:

`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.3/solang.exe>`_

Using hyperledgerlabs/solang docker hub images
----------------------------------------------

New images are automatically made available on
`docker hub <https://hub.docker.com/repository/docker/hyperledgerlabs/solang/>`_. 
Simply pull the `latest` tag docker image using:

.. code-block:: bash

	docker pull hyperledgerlabs/solang

And if you are using podman:

.. code-block:: bash

	podman image pull hyperlederlabs/solang

The Solang binary is in ``/usr/bin/solang`` in this image. The `latest` tag
gets updated each time there is a commit to the master branch of the solang
git repository.

Build Solang from source using Dockerfile
-----------------------------------------

First clone the git repo using:

.. code-block:: bash

  git clone https://github.com/hyperledger-labs/solang

Then you can build the image using:

.. code-block:: bash

	docker build .

Alternatively this will work with podman too:

.. code-block:: bash

	podman image build .

Building Solang from source from crates.io
------------------------------------------

The latest solang release is  on `crates.io <https://crates.io/crates/solang>`_.

Building Solang from source
---------------------------

In order to build solang from source, you will need rust 1.42.0 or higher,
and llvm version 10 with the WebAssembly target enabled.

So see if you have the correct version of rust, simply execute:

.. code-block:: bash

  rustc --version

If you do not have the correct version of rust installed, go to `rustup <https://rustup.rs/>`_.

After making sure llvm and rust are installed, just run:

.. code-block:: bash

  cargo build --release

The executable will be in ``target/release/solang``.

Verify that you have the LLVM Libraries installed
-------------------------------------------------

To make sure you have the correct version of the llvm libraries installed, first run:

.. code-block:: bash

  llvm-config --version

The output should be 10.0. Then check if the WebAssembly target is enabled by running:

.. code-block:: bash

  llc --version

You should see wasm32 listed under the targets. Lastly check that the static libraries are installed:

.. code-block:: bash

  llvm-config --link-static --libs

If there is no output, there are no static llvm libraries and building will fail.

Installing the LLVM Libraries
-----------------------------

If you do not have the llvm libraries installed then you can either install
your distribution llvm packages, or compile your own. Compiling your own is helpful
if you want to do Solang development.

Any build of llvm 10.0, with the WebAssembly target enabled, should work.
Note that you will also need clang; the Solidity standard library is written in C,
and is compiled to wasm by clang. The version of clang *must* be the same as the
version of llvm.


Installing LLVM on Ubuntu
_________________________

You will need ubuntu 20.04 (focal) or later. Just run:

.. code-block:: bash

	sudo apt install curl llvm-10-dev clang-10 git zlib1g-dev cargo

Installing LLVM on Debian
_________________________

You will need at least Debian Bullseye (testing).

.. code-block:: bash

	sudo apt-get install llvm-10-dev clang-10 zlib1g-dev pkg-config libssl-dev git cargo

Installing LLVM on Fedora
_________________________

You will need Fedora 32 or later. Running the following:

.. code-block:: bash

	sudo dnf install cargo llvm-static llvm-devel zlib-devel clang libffi-devel openssl-devel git

.. _llvm-from-source:

Installing LLVM on Windows
__________________________

A pre-built version of llvm, specifically configured for Solang, is available on
`solang.io <https://solang.io/download/llvm10.0.zip>`_. This binary is built using
the dockerfile used in `Building LLVM using Windows Containers`_. After unzipping
the file, add the bin directory to your path.

.. code-block::

	set PATH=%PATH%;C:\llvm10.0\bin

Building LLVM from source
___________________________

If your distribution does not have the correct llvm library versions, then you have
to build your own. Building your own llvm libraries does not interfere with any llvm libraries
installed by your distribution.

The llvm project is a large code base so it will take some time to build.

If you are planning to do development on Solang itself, then building
llvm libraries can be helpful, see `Debugging issues with LLVM`.

The llvm project itself has a guide to `installing from source <http://www.llvm.org/docs/CMake.html>`_ which you may need to consult.
First if all clone the llvm repository:

.. code-block:: bash

	git clone git://github.com/llvm/llvm-project
	cd llvm-project

Now switch to the 10.0 release branch:

.. code-block:: bash

	git checkout -b release_10.x origin/release/10.x

Now run cmake to create the makefiles. Replace the *installdir* argument to ``CMAKE_INSTALL_PREFIX`` with with a directory where you would like to have llvm installed, and then run the build:

.. code-block:: bash

	cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_PROJECTS=clang  \
		-DLLVM_ENABLE_TERMINFO=Off -DCMAKE_BUILD_TYPE=Release \
		-DCMAKE_INSTALL_PREFIX=installdir -B build llvm
	cmake --build build --target install

Once the build has succeeded, the *installdir*/bin has to be added to your path so the
Solang build can find the ``llvm-config`` from this build:

.. code-block:: bash

	export PATH=installdir/bin:$PATH

And on Windows, assuming *installdir* was ``C:\Users\User\solang-llvm``:

.. code-block::

	set PATH=%PATH%;C:\Users\User\solang-llvm\bin


Building LLVM using docker
__________________________

You can build llvm using docker. A `dockerfile for building llvm on linux <https://github.com/hyperledger-labs/solang/blob/master/scripts/build-llvm-linux.dockerfile>`_
is in Solang github repo. Simply run the dockerfile:

.. code-block:: bash

	docker build -f build-llvm-linux.dockerfile .

This will take a few hours. The result will be an image with llvm compressed in ``/llvm10.0.tar.bz2``.


Building LLVM using Windows Containers
______________________________________

You can build llvm using Windows Containers. This requires `Docker Desktop <https://www.docker.com/products/docker-desktop>`_
and switch to `windows containers <https://docs.docker.com/docker-for-windows/#switch-between-windows-and-linux-containers>`_.
Docker on Windows needs Hyper-V. If you are running Windows 10 in a virtual machine, be sure to check
`this blog post <https://www.mess.org/2020/06/22/Hyper-V-in-KVM/>`_.

The `dockerfile for building llvm on Windows <https://github.com/hyperledger-labs/solang/blob/master/scripts/build-llvm-windows.dockerfile>`_
is in Solang github repo. Simply run the dockerfile:

.. code-block:: bash

	docker build -f build-llvm-windows.dockerfile .

This will take a few hours. The result will be an image with llvm compressed in ``c:\llvm10.0.zip``.
