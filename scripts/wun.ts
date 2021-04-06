function run() {
  const code = Deno.readFileSync("tests/data/output/program.wasm");

  type programFun = () => void;

  WebAssembly
    .instantiate(code, {
      imports: {
        writeln_int: writeln,
        writeln_real: writeln,
      },
    }).then((r) => {
      const program = r.instance.exports.program as programFun;
      program();
    });
}

function writeln(num: number) {
  console.log(num);
}

run();
