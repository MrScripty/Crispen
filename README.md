# Crispen

A Rust color-grading toolkit with a desktop demo. Load an image, shape its grade with wheels and curves, and check the result against video scopes. Embed the grading engine in another application, or explore it through the Bevy + Svelte interface.

Crispen is being developed as the color module for Studio Whip. It brings together grading math, GPU LUT processing, OpenColorIO color management, and OpenImageIO image loading. OpenFX integration currently covers plugin discovery and descriptor loading.

## Run the desktop demo

The launcher builds the Svelte interface, Rust application, and CEF helper, then starts the app. The setup below follows its Linux path.

Install Rust stable and Node.js with npm. On Ubuntu/Debian, install the native color libraries and their discovery tool:

```bash
sudo apt install pkg-config libopencolorio-dev libopenimageio-dev
git clone https://github.com/MrScripty/Crispen.git
cd Crispen
./launcher.sh
```

You also need a working desktop graphics environment and the platform build dependencies for Bevy and CEF. The default demo enables both CEF and OCIO/OIIO.

To work on the interface with hot reload, start Vite in one terminal:

```bash
cd crates/crispen-demo/ui
npm install
npm run dev
```

Then, from the repository root in another terminal:

```bash
./launcher.sh --dev
```

## Use the engine

- [crispen-core](crates/crispen-core/src/lib.rs): grading math, color transforms, LUTs, and scope calculations without GPU or framework dependencies
- [crispen-gpu](crates/crispen-gpu/src/README.md): GPU LUT baking and image processing
- [crispen-bevy](crates/crispen-bevy/src/README.md): the Bevy plugin and ECS integration
- [crispen-ocio](crates/crispen-ocio/README.md): OpenColorIO integration

## Development

```bash
make ci-lint
make ci-build
```

These check the full workspace with native color-library support. See [build options and troubleshooting](docs/building.md) for release builds, the native-UI fallback, and custom library paths.

## License

[MIT](LICENSE).
