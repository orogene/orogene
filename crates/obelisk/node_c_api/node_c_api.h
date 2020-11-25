#ifndef SRC_NODE_EXT_H_
#define SRC_NODE_EXT_H_

int node_main(int argc, char **argv);

extern "C"
{
  int execute_node(char *code);
}

#endif
