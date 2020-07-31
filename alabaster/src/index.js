module.exports = overrideNode();
function overrideNode() {
  require("./extensions").overrideNode();
}
overrideNode();
