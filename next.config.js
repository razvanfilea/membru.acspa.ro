const withPWA = require("next-pwa");

module.exports = withPWA({
  reactStrictMode: true,
  swcMinify: true,
  env: {
    REACT_APP_ENDPOINT: "https://server.aaconsl.com/v1",
    REACT_APP_PROJECT: "62cd41bedfc96abb0a49",
    REACT_APP_DATABASE_ID: "62cd4258a6ff315b2ca5"
  },
  pwa: {
    disable: process.env.NODE_ENV === 'development',
    // dest: "public",
    register: true,
    skipWaiting: true
  }
});
