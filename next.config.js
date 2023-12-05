const runtimeCaching = require('next-pwa/cache');
const withPWA = require("next-pwa")({
    disable: process.env.NODE_ENV === 'development',
    dest: "public",
    skipWaiting: false,
    runtimeCaching
})

module.exports = withPWA({
    reactStrictMode: true,
    output: 'export',

    webpack: (config, { isServer }) => {
        if (!isServer) {
            Object.assign(config.resolve.alias, {
                "react/jsx-runtime.js": "preact/compat/jsx-runtime",
                react: "preact/compat",
                "react-dom/test-utils": "preact/test-utils",
                "react-dom": "preact/compat",
            });
        }
        return config;
    },
});
