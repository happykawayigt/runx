// Inputs arrive as RUNX_INPUT_<NAME> environment variables. Do the work and
// write the result to stdout. Replace this echo with the real logic.
const message = process.env.RUNX_INPUT_MESSAGE ?? "";
if (message.trim().length === 0) {
  process.stderr.write("message is required\n");
  process.exit(64);
}
process.stdout.write(`${message}\n`);
