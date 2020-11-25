import fs from "fs";
import path from "path";
import { parse } from "gyp-parser";

const NODE_PATH = "vendor/node";

const gyp_exists = fs.existsSync(path.resolve(`${NODE_PATH}/node.gyp`));

if (!gyp_exists) {
  throw Error(
    "node.gyp not found! Make sure to fetch files from node repository first!"
  );
}

// Inject C++ embedder API into Node's source

console.log("Copying C++ embedder API files to Node's source...");

const copy_src = ["node_c_api.cc", "node_c_api.h"];

for (let src of copy_src) {
  fs.copyFileSync(
    path.resolve("node_c_api", src),
    path.resolve(`${NODE_PATH}/src/${src}`)
  );
}

// Add copied files to gyp build configuration

console.log("Reading and modifying node.gyp...");

const raw_gyp = fs.readFileSync(`${NODE_PATH}/node.gyp`, { encoding: "utf8" });
const gyp = parse(raw_gyp);

const cpp_source_list = gyp.targets.find(
  (t) => t.target_name === "<(node_lib_target_name)"
).sources;

cpp_source_list.push("src/node_c_api.cc", "src/node_c_api.h");

const serialized = JSON.stringify(gyp, null, 2);
fs.writeFileSync(`${NODE_PATH}/node.gyp`, serialized, { encoding: "utf8" });

console.log("SUCCESS!");
