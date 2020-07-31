const babel = require("@babel/core");
const reactPlugin = require("@babel/plugin-transform-react-jsx");
const tsPlugin = require("@babel/plugin-transform-typescript");
const fs = require("fs");
const Module = require("module");

module.exports.overrideNode = overrideNode;
function overrideNode() {
  // These modules are lazy-loaded.
  Module._extensions[".jsx"] = (module, filename) => {
    const content = fs.readFileSync(filename, "utf8");
    const { code } = babel.transform(content, {
      plugins: [
        [
          reactPlugin,
          {
            useBuiltIns: true
          }
        ]
      ]
    });
    module._compile(code, filename);
  };

  Module._extensions[".ts"] = (module, filename) => {
    const content = fs.readFileSync(filename, "utf8");
    const { code } = babel.transform(content, {
      plugins: [tsPlugin]
    });
    module._compile(code, filename);
  };

  Module._extensions[".tsx"] = Module._extensions[".ts"];
}
