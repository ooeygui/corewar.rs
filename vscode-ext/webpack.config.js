const path = require('path');

/** @type {import('webpack').Configuration} */
const sharedConfig = {
  mode: 'none',
  resolve: {
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        exclude: /node_modules/,
        use: [
          {
            loader: 'ts-loader'
          }
        ]
      }
    ]
  },
  devtool: 'nosources-source-map'
};

/** @type {import('webpack').Configuration[]} */
module.exports = [
  {
    ...sharedConfig,
    target: 'node',
    entry: './src/extension.ts',
    output: {
      path: path.resolve(__dirname, 'dist'),
      filename: 'extension.js',
      libraryTarget: 'commonjs2',
      devtoolModuleFilenameTemplate: '../[resource-path]'
    },
    externals: {
      vscode: 'commonjs vscode'
    }
  },
  {
    ...sharedConfig,
    target: 'web',
    entry: './src/webview/main.ts',
    output: {
      path: path.resolve(__dirname, 'dist'),
      filename: 'webview.js',
      devtoolModuleFilenameTemplate: '../[resource-path]'
    }
  }
];
