# escape=`

# Use the latest Windows Server Core image
FROM mcr.microsoft.com/windows/servercore:ltsc2019

SHELL [ "powershell", "-Command", "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'Continue'; $verbosePreference='Continue';"]

# Download Visual Studio Build Tools 16.8.3. This should match the version on github actions virtual environment.
# https://docs.microsoft.com/en-us/visualstudio/releases/2019/history
# https://github.com/actions/virtual-environments/blob/main/images/win/Windows2019-Readme.md
ADD https://download.visualstudio.microsoft.com/download/pr/9b3476ff-6d0a-4ff8-956d-270147f21cd4/0df5becfebf4ae2418f5fae653feebf3888b0af00d3df0415cb64875147e9be3/vs_BuildTools.exe C:\TEMP\vs_buildtools.exe

# Install Visual Studio Build Tools
RUN C:\TEMP\vs_buildtools.exe --quiet --wait --norestart --nocache `
	--installPath C:\BuildTools `
	--add Microsoft.VisualStudio.Component.VC.CMake.Project `
	--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
	--add Microsoft.VisualStudio.Component.VC.ATL `
	--add Microsoft.VisualStudio.Component.Windows10SDK.18362

# Rust
ADD https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe C:\TEMP\rustup-init.exe

RUN C:\TEMP\rustup-init.exe -y

# Git
ADD https://github.com/git-for-windows/git/releases/download/v2.30.0.windows.1/MinGit-2.30.0-64-bit.zip C:\TEMP\MinGit-2.30.0-64-bit.zip

RUN Expand-Archive C:\TEMP\MinGit-2.30.0-64-bit.zip -DestinationPath c:\MinGit

# LLVM Build requires Python
# Newer versions than v3.5.4 fail due to https://github.com/microsoft/vcpkg/issues/6988
ADD https://www.python.org/ftp/python/3.5.4/python-3.5.4-embed-amd64.zip C:\TEMP\python-3.5.4-embed-amd64.zip

RUN Expand-Archive C:\TEMP\python-3.5.4-embed-amd64.zip -DestinationPath c:\Python

# PowerShell community extensions needed for Invoke-BatchFile
# Update Compress-Archive so that slashes are used: https://github.com/PowerShell/Microsoft.PowerShell.Archive/issues/71
RUN Install-PackageProvider -Name NuGet -MinimumVersion 2.8.5.201 -Force ; `
	Install-Module -name Pscx -Scope CurrentUser -Force -AllowClobber ; `
	Install-Module -name Microsoft.PowerShell.Archive -MinimumVersion 1.2.3.0 -Repository PSGallery -Force -AllowClobber

# Invoke-BatchFile retains the environment after executing so we can set it up more permanently
RUN Invoke-BatchFile C:\BuildTools\vc\Auxiliary\Build\vcvars64.bat ; `
	$path = $env:path + ';c:\MinGit\cmd;C:\Users\ContainerAdministrator\.cargo\bin;C:\llvm11.0\bin;C:\Python' ; `
	Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment\' -Name Path -Value $path ; `
	Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment\' -Name LIB -Value $env:LIB ; `
	Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment\' -Name INCLUDE -Value $env:INCLUDE ; `
	Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment\' -Name LIBPATH -Value $env:LIBPATH ;

RUN git clone --single-branch git://github.com/solana-labs/llvm-project

WORKDIR \llvm-project

# Stop cmake from re-generating build system ad infinitum
RUN Add-Content llvm\CMakeLists.txt 'set(CMAKE_SUPPRESS_REGENERATION 1)' ;

# All llvm targets should be enabled or inkwell refused to link
RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On '-DLLVM_ENABLE_PROJECTS=clang;lld' `
	-DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=C:/llvm11.0 `
	-B build llvm
RUN cmake --build build --target install

WORKDIR \

RUN Compress-Archive -Path C:\llvm11.0 -DestinationPath C:\llvm11.0-win.zip

RUN Remove-Item -Path \llvm-project,C:\TEMP -Recurse -Force
