extern "C"
{
  int execute_node(char *code);

  int run_node(char *code)
  {
    return execute_node(code);
  }
}
