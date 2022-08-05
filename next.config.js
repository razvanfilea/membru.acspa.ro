const withPWA = require("next-pwa");

module.exports = withPWA({
  reactStrictMode: true,
  swcMinify: true,
  pwa: {
    disable: process.env.NODE_ENV === 'development',
    // dest: "public",
    register: true,
    skipWaiting: true
  }
});
