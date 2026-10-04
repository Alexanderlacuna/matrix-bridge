#!/usr/bin/env python3
"""Grab a Matrix SSO login token for `matrix-bridge login-token`.

Opens the browser at the homeserver's SSO redirect URL with
redirectUrl pointed at a temporary localhost listener; after you log in
with your identity provider, the provider redirects the browser here with
?loginToken=... which we print.

The login token is single-use and expires within minutes — feed it to
`matrix-bridge login-token` immediately.

Usage:
    python3 sso-login-token.py [--homeserver https://matrix-client.matrix.org]
                               [--port 8765]
"""
import argparse
import http.server
import urllib.parse
import urllib.request
import webbrowser


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--homeserver", default="https://matrix-client.matrix.org")
    ap.add_argument("--port", type=int, default=8765)
    args = ap.parse_args()

    redirect = f"http://localhost:{args.port}/callback"
    url = (
        f"{args.homeserver}/_matrix/client/v3/login/sso/redirect"
        f"?redirectUrl={urllib.parse.quote(redirect, safe='')}"
    )

    got = {}

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802 - stdlib API
            q = urllib.parse.urlparse(self.path)
            if q.path != "/callback":
                self.send_response(404)
                self.end_headers()
                return
            params = urllib.parse.parse_qs(q.query)
            token = params.get("loginToken", [None])[0]
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.end_headers()
            if token:
                got["token"] = token
                self.wfile.write(
                    b"Login token captured — you can close this tab and "
                    b"return to the terminal.\n"
                )
            else:
                self.wfile.write(
                    b"No loginToken in the redirect. Check the URL you were "
                    b"sent and try again.\n"
                )

        def log_message(self, fmt, *fmtargs):  # silence default logging
            pass

    server = http.server.HTTPServer(("127.0.0.1", args.port), Handler)
    server.timeout = 600

    print(f"Opening SSO login in your browser: {url}")
    print("Log in with your provider; the tab will say when the token is captured.")
    try:
        webbrowser.open(url)
    except webbrowser.Error:
        print(f"(could not open a browser automatically — visit the URL above)")

    while "token" not in got:
        server.handle_request()

    server.server_close()
    print()
    print(f"LOGIN TOKEN: {got['token']}")
    print("Now run:  matrix-bridge login-token '<LOGIN TOKEN>'")


if __name__ == "__main__":
    main()
