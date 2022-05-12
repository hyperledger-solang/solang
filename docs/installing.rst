Installing Solang
=================

The Solang compiler is a single binary. It can be installed in different ways.

Download release binaries
-------------------------

There are binaries available on github releases:

- `Linux x86-64 <https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/solang-linux-x86-64>`_
- `Linux arm64 <https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/solang-linux-arm64>`_
- `Windows x64 <https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/solang.exe>`_
- `MacOS intel <https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/solang-mac-intel>`_
- `MacOS arm <https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/solang-mac-arm>`_

On MacOS, remember to remove the quarantine attribute using ``xattr -d com.apple.quarantine solang-mac-arm``
in the terminal.

Using ghcr.io/hyperledger-labs/solang containers
------------------------------------------------

New images are automatically made available on
`solang containers <https://github.com/hyperledger-labs/solang/pkgs/container/solang>`_.
There is a release `v0.1.11` tag and a `latest` tag:

.. code-block:: bash

	docker pull ghcr.io/hyperledger-labs/solang:latest

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

In order to build Solang from source, you will need rust 1.59.0 or higher,
and a build of llvm based on the Solana llvm tree. There are a few patches which are not upstream yet.
First, follow the steps below for installing llvm and then proceed from there.

If you do not have the correct version of rust installed, go to `rustup <https://rustup.rs/>`_.

Installing the LLVM Libraries
-----------------------------

Solang needs a build of
`llvm with some extra patches <https://github.com/solana-labs/llvm-project/>`_.
These patches make it possible to generate code for Solana, and fixes some
concurrency issues in the lld linker.

You can either download the pre-built libraries from
`github <https://github.com/hyperledger-labs/solang/releases/tag/v0.1.11>`_
or build your own from source. After that, you need to add the `bin` directory to your
path, so that the build system of Solang can find the correct version of llvm to use.

Installing LLVM on Linux
________________________

A pre-built version of llvm, specifically configured for Solang, is available at
`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/llvm13.0-linux-x86-64.tar.xz>`_.
After downloading, untar the file in a terminal and add it to your path.

.. code-block:: bash

	tar Jxf llvm13.0-linux-x86-64.tar.xz
	export PATH=$(pwd)/llvm13.0/bin:$PATH

Installing LLVM on Windows
__________________________

A pre-built version of llvm, specifically configured for Solang, is available at
`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/llvm13.0-win.zip>`_.

After unzipping the file, add the bin directory to your path.

.. code-block:: batch

	set PATH=%PATH%;C:\llvm13.0\bin

Installing LLVM on Mac
______________________

A pre-built version of llvm for intel macs, is available at
`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/llvm13.0-mac-intel.tar.xz>`_ and for arm macs there is
`<https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/llvm13.0-mac-arm.tar.xz>`_. After downloading,
untar the file in a terminal and add it to your path like so:

.. code-block:: bash

	tar Jxf llvm13.0-mac-arm.tar.xz
	xattr -rd com.apple.quarantine llvm13.0
	export PATH=$(pwd)/llvm13.0/bin:$PATH

.. _llvm-from-source:

Building LLVM from source
___________________________

The llvm project itself has a guide to `installing from source <http://www.llvm.org/docs/CMake.html>`_ which
you may need to consult. First if all clone our llvm repository:

.. code-block:: bash

	git clone --depth 1 --branch solana-rustc/13.0-2021-08-08 https://github.com/solana-labs/llvm-project
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
