Installing Solang
=================

The Solang compiler is a single binary. It can be installed in different ways.

Download release binaries
-------------------------

For Linux x86-64, there is a binary available in the github releases:

`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.7/solang-linux>`_

For Windows x64, there is a binary available:

`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.7/solang.exe>`_

For MacOS, there is an intel binary available.
Remember to remove the quarantine attribute using ``xattr -d com.apple.quarantine solang-mac`` in the terminal.

`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.7/solang-mac>`_


Using hyperledgerlabs/solang docker hub images
----------------------------------------------

New images are automatically made available on
`docker hub <https://hub.docker.com/repository/docker/hyperledgerlabs/solang/>`_.
There is a release `v0.1.7` tag and a `latest` tag:

.. code-block:: bash

	docker pull hyperledgerlabs/solang

The Solang binary is stored at ``/usr/bin/solang`` in this image. The `latest` tag
gets updated each time there is a commit to the main branch of the Solang git repository.

Build Solang using Dockerfile
-----------------------------

First clone the git repo using:

.. code-block:: bash

  git clone https://github.com/hyperledger-labs/solang

Then you can build the image using:

.. code-block:: bash

	docker image build .

Building Solang from source
---------------------------

In order to build Solang from source, you will need rust 1.53.0 or higher,
and a build of llvm based on the Solana llvm tree. There are a few patches which are not upstream yet.
First, follow the steps below for installing llvm and then proceed from there.

If you do not have the correct version of rust installed, go to `rustup <https://rustup.rs/>`_.

Installing the LLVM Libraries
-----------------------------

Solang needs a build of
`llvm with some extra patches <https://github.com/solana-labs/llvm-project/>`_.
You can either download the pre-built binaries or build your own from source. After that,
You need to add the `bin` directory to your path, so that the build system of Solang can find the
correct version of llvm to use.

Installing LLVM on Linux
________________________

A pre-built version of llvm, specifically configured for Solang, is available at
`<https://solang.io/download/llvm11.0-linux.tar.xz>`_ for x64 and
`<https://solang.io/download/llvm11.0-linux-arm64.tar.xz>`_ for arm64. This version is built using the
`dockerfile for building llvm on linux <https://github.com/hyperledger-labs/solang/blob/main/scripts/build-llvm-linux.dockerfile>`_.
After downloading, untar the file in a terminal and add it to your path.

.. code-block:: bash

	tar Jxf llvm11.0-linux.tar.xz
	export PATH=$(pwd)/llvm11.0/bin:$PATH

Installing LLVM on Windows
__________________________

A pre-built version of llvm, specifically configured for Solang, is available at
`<https://solang.io/download/llvm11.0-win.zip>`_. This version is built using the
`dockerfile for building llvm on Windows <https://github.com/hyperledger-labs/solang/blob/main/scripts/build-llvm-windows.dockerfile>`_.

If you want to use the dockerfile yourself rather than download the binaries above, then this
requires `Docker Desktop <https://www.docker.com/products/docker-desktop>`_ installed, and then switched to
`windows containers <https://docs.docker.com/docker-for-windows/#switch-between-windows-and-linux-containers>`_.
The result will be an image with llvm compressed in the file ``c:\llvm11.0-win.zip``. Docker on Windows needs Hyper-V
enabled. If you are running Windows 10 in a virtual machine, be sure to check
`this blog post <https://www.mess.org/2020/06/22/Hyper-V-in-KVM/>`_.

After unzipping the file, add the bin directory to your path.

.. code-block:: batch

	set PATH=%PATH%;C:\llvm11.0\bin

Installing LLVM on Mac
______________________

A pre-built version of llvm for intel macs, is available at
`<https://solang.io/download/llvm11.0-mac.tar.xz>`_. If you have an arm/m1 mac,
then you have to build your own llvm using the instructions below. After downloading,
untar the file in a terminal and add it to your path like so:

.. code-block:: bash

	tar Jxf llvm11.0-mac.tar.xz
	xattr -rd com.apple.quarantine llvm11.0
	export PATH=$(pwd)/llvm11.0/bin:$PATH

.. _llvm-from-source:

Building LLVM from source
___________________________

The llvm project itself has a guide to `installing from source <http://www.llvm.org/docs/CMake.html>`_ which
you may need to consult. First if all clone our llvm repository:

.. code-block:: bash

	git clone --branch solana-rustc/11.0-2021-01-05 git://github.com/solana-labs/llvm-project
	cd llvm-project

Now run cmake to create the makefiles. Replace the *installdir* argument to ``CMAKE_INSTALL_PREFIX`` with with a directory where you would like to have llvm installed, and then run the build:

.. code-block:: bash

	cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On '-DLLVM_ENABLE_PROJECTS=clang;lld'  \
		-DLLVM_ENABLE_TERMINFO=Off -DCMAKE_BUILD_TYPE=Release \
		-DCMAKE_INSTALL_PREFIX=installdir -B build llvm
	cmake --build build --target install

Once the build has succeeded, the *installdir*/bin has to be added to your path so the
Solang build can find the ``llvm-config`` from this build:

.. code-block:: bash

	export PATH=installdir/bin:$PATH

And on Windows, assuming *installdir* was ``C:\Users\User\solang-llvm``:

.. code-block:: batch

	set PATH=%PATH%;C:\Users\User\solang-llvm\bin

Building Solang from crates.io
------------------------------

The latest Solang release is  on `crates.io <https://crates.io/crates/solang>`_. Once you have the
correct llvm version in your path, simply run:

.. code-block:: bash

	cargo install solang

Building Solang from git
------------------------

Once you have the correct llvm version in your path, simply run:

.. code-block:: bash

	git clone https://github.com/hyperledger-labs/solang/
	cd solang
	cargo build --release

The executable will be in ``target/release/solang``.
