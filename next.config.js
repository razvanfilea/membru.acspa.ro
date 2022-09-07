const runtimeCaching = require('next-pwa/cache');
const withPWA = require("next-pwa")({
    disable: process.env.NODE_ENV === 'development',
    dest: "public",
    skipWaiting: false,
    runtimeCaching
})

module.exports = withPWA({
    reactStrictMode: true,
    swcMinify: true
});
