import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { generateEmailVerifierInputs } from "@zk-email/helpers/dist/input-generators.js";

async function main() {
  const [, , emailPath, outputPath] = process.argv;

  if (!emailPath || !outputPath) {
    throw new Error("usage: generate_inputs.ts <email-path> <output-path>");
  }

  const rawEmail = readFileSync(resolve(emailPath), "utf8");
  const inputs = await generateEmailVerifierInputs(
    rawEmail,
    {
      maxHeadersLength: 576,
      maxBodyLength: 64,
      ignoreBodyHashCheck: true,
    },
    {
      fallbackToZKEmailDNSArchive: true,
    },
  );

  writeFileSync(resolve(outputPath), `${JSON.stringify(inputs, null, 2)}\n`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
