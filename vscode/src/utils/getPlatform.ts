export default function getPlatform(): string | undefined {
  switch (`${process.arch} ${process.platform}`) {
    case 'x64 win32':
    case 'arm64 win32':
      return 'solang.exe';
    case 'x64 linux':
      return 'solang-linux-x86-64';
    case 'arm64 linux':
      return 'solang-linux-arm64';
    case 'x64 darwin':
      return 'solang-mac-intel';
    case 'arm64 darwin':
      return 'solang-mac-arm';
    default:
      return;
  }
}
