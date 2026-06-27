// smol.js — OpenCode plugin
// Auto-wraps Bash commands with smol for summarized output
module.exports = {
  name: 'smol',
  preToolUse(tool, input) {
    if (tool !== 'Bash' || !input || !input.command) return input;
    const cmd = input.command.trim();
    if (cmd.startsWith('smol ') || cmd.startsWith('cd ') || cmd.startsWith('exit') || cmd.startsWith('export ') || cmd.startsWith('ls ') || cmd.startsWith('cat ') || cmd.startsWith('echo ') || cmd.startsWith('head ') || cmd.startsWith('tail ') || cmd.length <= 10) {
      return input;
    }
    input.command = `smol --sync ${cmd}`;
    return input;
  }
};
