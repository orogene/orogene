# Orogene - Antimony

## Development requirements

Before attempting to run this project, you should have both node and rust installed on your machine

```
rust    ^1.45
node    ^12.0
```

## Working directory

The commands below assume the current working directory is `/antimony`

## Project setup

#### Build the main rust project from the repo's root directory

```bash
cd ../
cargo build
cd antimony
```

#### install npm dependencies

```bash
npm install
```

### Compiles and hot-reloads for development

```bash
npm run tauri:serve
```

### Compiles and minifies for production

```bash
npm run tauri:build
```

### Lints and fixes files

```bash
npm run lint
```

### Customize configuration

See [Configuration Reference](https://cli.vuejs.org/config/).
