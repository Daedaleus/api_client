// Stands in for the user's browser in the end-to-end test (started through
// the BROWSER variable): opens the sign-in URL given as last argument and
// follows its redirect back to the app, as a browser would after signing in.
const url = process.argv.at(-1);
const response = await fetch(url, { redirect: 'follow' });
process.exit(response.ok ? 0 : 1);
