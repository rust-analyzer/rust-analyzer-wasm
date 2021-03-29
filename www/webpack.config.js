const path = require("path");
const HtmlWebPackPlugin = require("html-webpack-plugin");

module.exports = {
    mode: "development",
    entry: {
        app: "./index.js",
        ra: "./ra-worker.js",
        "editor.worker": "monaco-editor/esm/vs/editor/editor.worker.js",
    },
    output: {
        globalObject: "self",
        filename: "[name].bundle.js",
        path: path.resolve(__dirname, "dist"),
    },
    module: {
        rules: [
            {
                test: /\.css$/,
                use: ["style-loader", "css-loader"],
            },
            {
                test: /\.ttf$/,
                use: ['file-loader']
            }
        ],
    },
    plugins: [
        new HtmlWebPackPlugin({
            title: "Rust Analyzer",
            chunks: ["app"],
        }),
    ],    
    // It is needed for firefox works
    devServer: {
        headers: {
            'Cross-Origin-Embedder-Policy': 'require-corp',
            'Cross-Origin-Opener-Policy': 'same-origin'
        },
    },
};
