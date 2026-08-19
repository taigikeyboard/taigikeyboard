#!/usr/bin/env python3
# Stdlib-only Gmail triage helper for Taigi Keyboard user bug reports.
# No third-party deps (no google-auth) so it survives a clean machine.
# Auth = OAuth installed-app loopback with client_secret.json; token cached
# with refresh_token so re-login is rare. Scope = gmail.modify (apply labels).
#
# Commands:
#   auth                 ensure a valid access token (refresh or full login)
#   list [n]             list UNFIXED bug reports (default 20), payment noise skipped
#   body <id>            print headers + plaintext body of one message
#   fixed <id> [<id>..]  add the "開發/Bug Report/Fixed" label to message(s)
#   labels               list all Gmail labels (debug — verify names/ids)

import base64
import json
import os
import socket
import sys
import urllib.error
import urllib.parse
import urllib.request
import webbrowser
from http.server import BaseHTTPRequestHandler, HTTPServer

SCOPE = "https://www.googleapis.com/auth/gmail.modify"
TOKEN_PATH = os.path.expanduser("~/.config/gmail-cli/token.json")
API = "https://gmail.googleapis.com/gmail/v1/users/me"

BUG_LABEL = "開發/問題回報"
FIXED_LABEL = "開發/問題回報/已修復"
UNRESOLVED_LABEL = "開發/問題回報/無法重現"
FEATURE_LABEL = "開發/功能建議"
# This label also auto-catches payment mail — skip those when listing.
PAYMENT_NOISE = ("ecpay", "藍新", "newebpay", "發票", "invoice", "付款通知")


def _repo_root():
    # script lives at <repo>/.claude/skills/bug-triage/gmail_cli.py
    return os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))


def _client_secret_path():
    env = os.environ.get("GMAIL_CLIENT_SECRET")
    if env and os.path.exists(env):
        return env
    candidate = os.path.join(_repo_root(), "client_secret.json")
    if os.path.exists(candidate):
        return candidate
    sys.exit(
        "client_secret.json not found. Put the OAuth installed-app secret at "
        f"{candidate} (gitignored) or set GMAIL_CLIENT_SECRET."
    )


def _client():
    data = json.load(open(_client_secret_path()))
    return data.get("installed") or data.get("web")


def _load_token():
    if os.path.exists(TOKEN_PATH):
        return json.load(open(TOKEN_PATH))
    return None


def _save_token(tok):
    os.makedirs(os.path.dirname(TOKEN_PATH), exist_ok=True)
    with open(TOKEN_PATH, "w") as f:
        json.dump(tok, f)
    os.chmod(TOKEN_PATH, 0o600)


def _post_form(url, fields):
    body = urllib.parse.urlencode(fields).encode()
    req = urllib.request.Request(url, data=body, method="POST")
    with urllib.request.urlopen(req) as r:
        return json.load(r)


def _refresh(tok):
    c = _client()
    resp = _post_form(
        c["token_uri"],
        {
            "grant_type": "refresh_token",
            "refresh_token": tok["refresh_token"],
            "client_id": c["client_id"],
            "client_secret": c["client_secret"],
        },
    )
    tok["token"] = resp["access_token"]
    _save_token(tok)
    return tok


def _full_login():
    c = _client()
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.bind(("127.0.0.1", 0))
    port = sock.getsockname()[1]
    sock.close()
    redirect_uri = f"http://localhost:{port}/"

    auth_url = c["auth_uri"] + "?" + urllib.parse.urlencode(
        {
            "response_type": "code",
            "client_id": c["client_id"],
            "redirect_uri": redirect_uri,
            "scope": SCOPE,
            "access_type": "offline",
            "prompt": "consent",
        }
    )

    captured = {}

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            qs = urllib.parse.urlparse(self.path).query
            captured.update(urllib.parse.parse_qs(qs))
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.end_headers()
            self.wfile.write("Auth done. Close this tab.".encode())

        def log_message(self, *_):
            pass

    print("Opening browser for Gmail consent…", file=sys.stderr)
    print(auth_url, file=sys.stderr)
    webbrowser.open(auth_url)
    httpd = HTTPServer(("127.0.0.1", port), Handler)
    httpd.handle_request()
    code = captured.get("code", [None])[0]
    if not code:
        sys.exit("OAuth failed — no code returned.")

    resp = _post_form(
        c["token_uri"],
        {
            "grant_type": "authorization_code",
            "code": code,
            "client_id": c["client_id"],
            "client_secret": c["client_secret"],
            "redirect_uri": redirect_uri,
        },
    )
    tok = {
        "token": resp["access_token"],
        "refresh_token": resp.get("refresh_token"),
        "scopes": [SCOPE],
        "token_uri": c["token_uri"],
        "client_id": c["client_id"],
    }
    _save_token(tok)
    return tok


def ensure_token():
    tok = _load_token()
    if tok and tok.get("refresh_token"):
        try:
            return _refresh(tok)  # always refresh → fresh access token
        except urllib.error.HTTPError as e:
            # invalid_grant: refresh_token expired (OAuth app in "testing"
            # mode expires them after 7 days) or revoked → re-consent.
            print(f"refresh failed ({e.code}) — re-running login flow…", file=sys.stderr)
    return _full_login()


def _api(path, tok, method="GET", payload=None, params=None):
    url = API + path
    if params:
        url += "?" + urllib.parse.urlencode(params, doseq=True)
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", "Bearer " + tok["token"])
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req) as r:
            raw = r.read()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        if e.code == 401:  # access token expired mid-run → refresh once + retry
            _refresh(tok)
            req2 = urllib.request.Request(url, data=data, method=method)
            req2.add_header("Authorization", "Bearer " + tok["token"])
            if data is not None:
                req2.add_header("Content-Type", "application/json")
            with urllib.request.urlopen(req2) as r:
                raw = r.read()
                return json.loads(raw) if raw else {}
        sys.exit(f"Gmail API {e.code}: {e.read().decode(errors='replace')}")


def _label_map(tok):
    labels = _api("/labels", tok).get("labels", [])
    return {lb["name"]: lb["id"] for lb in labels}


def _header(headers, name):
    for h in headers:
        if h.get("name", "").lower() == name.lower():
            return h.get("value", "")
    return ""


# Deep-link to a single message in the Gmail web UI. `#all/<messageId>` opens the
# message regardless of which label the user is currently browsing.
GMAIL_MESSAGE_URL = "https://mail.google.com/mail/u/0/#all/{msg_id}"


def _is_noise(frm, subj):
    blob = (frm + " " + subj).lower()
    return any(k.lower() in blob for k in PAYMENT_NOISE)


def cmd_list(tok, n):
    # Open bug queue = reported, not yet fixed, not yet categorized as a feature.
    query = (
        f'label:"{BUG_LABEL}" -label:"{FIXED_LABEL}" -label:"{FEATURE_LABEL}"'
        f' -label:"{UNRESOLVED_LABEL}"'
    )
    res = _api("/messages", tok, params={"q": query, "maxResults": n})
    msgs = res.get("messages", [])
    if not msgs:
        print("No unfixed bug reports.")
        return
    shown = skipped = 0
    for m in msgs:
        full = _api(
            f"/messages/{m['id']}",
            tok,
            params={
                "format": "metadata",
                "metadataHeaders": ["From", "Subject", "Date"],
            },
        )
        headers = full.get("payload", {}).get("headers", [])
        frm = _header(headers, "From")
        subj = _header(headers, "Subject")
        date = _header(headers, "Date")
        if _is_noise(frm, subj):
            skipped += 1
            continue
        shown += 1
        print(f"[{shown}] id={m['id']}")
        print(f"    date: {date}")
        print(f"    from: {frm}")
        print(f"    subj: {subj}")
        print(f"    link: {GMAIL_MESSAGE_URL.format(msg_id=m['id'])}")
        print(f"    snip: {full.get('snippet', '')[:200]}")
    print(f"\n({shown} unfixed report(s); {skipped} payment-noise skipped)")


def _walk_plaintext(payload, out):
    mime = payload.get("mimeType", "")
    body = payload.get("body", {})
    if mime == "text/plain" and body.get("data"):
        out.append(base64.urlsafe_b64decode(body["data"] + "===").decode(errors="replace"))
    for part in payload.get("parts", []) or []:
        _walk_plaintext(part, out)


def cmd_body(tok, msg_id):
    full = _api(f"/messages/{msg_id}", tok, params={"format": "full"})
    headers = full.get("payload", {}).get("headers", [])
    print("From:   ", _header(headers, "From"))
    print("Subject:", _header(headers, "Subject"))
    print("Date:   ", _header(headers, "Date"))
    print("Id:     ", msg_id)
    print("Link:   ", GMAIL_MESSAGE_URL.format(msg_id=msg_id))
    print("-" * 60)
    out = []
    _walk_plaintext(full.get("payload", {}), out)
    print("\n".join(out).strip() if out else full.get("snippet", ""))


def _apply_label(tok, label_name, ids, verb, remove_label_names=()):
    labels = _label_map(tok)
    label_id = labels.get(label_name)
    if not label_id:
        # Auto-create: hierarchy comes from the name ("parent/child").
        created = _api("/labels", tok, method="POST", payload={"name": label_name})
        label_id = created.get("id")
        if not label_id:
            sys.exit(f'Label "{label_name}" does not exist and could not be created.')
        print(f'created label "{label_name}"')
    remove_ids = [labels[n] for n in remove_label_names if n in labels]
    for msg_id in ids:
        _api(
            f"/messages/{msg_id}/modify",
            tok,
            method="POST",
            payload={"addLabelIds": [label_id], "removeLabelIds": remove_ids},
        )
        print(f"{verb}: {msg_id}")


def cmd_fixed(tok, ids):
    _apply_label(tok, FIXED_LABEL, ids, "marked fixed")


def cmd_feature(tok, ids):
    _apply_label(tok, FEATURE_LABEL, ids, "marked feature")


def cmd_unresolved(tok, ids):
    # NOT-repro / cannot-verify: the report stays acknowledged but leaves the
    # open queue; distinct from 已修復 (a merged fix exists). Clears a
    # previously (mis)applied 已修復 on the same message.
    _apply_label(tok, UNRESOLVED_LABEL, ids, "marked unresolved", remove_label_names=(FIXED_LABEL,))


def cmd_labels(tok):
    for name, lid in sorted(_label_map(tok).items()):
        print(f"{lid}\t{name}")


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: gmail_cli.py {auth|list [n]|body <id>|fixed <id..>|feature <id..>|unresolved <id..>|labels}")
    cmd = sys.argv[1]
    tok = ensure_token()
    if cmd == "auth":
        print("Token OK (scope gmail.modify).")
    elif cmd == "list":
        cmd_list(tok, int(sys.argv[2]) if len(sys.argv) > 2 else 20)
    elif cmd == "body":
        cmd_body(tok, sys.argv[2])
    elif cmd == "fixed":
        if len(sys.argv) < 3:
            sys.exit("usage: gmail_cli.py fixed <id> [<id>..]")
        cmd_fixed(tok, sys.argv[2:])
    elif cmd == "feature":
        if len(sys.argv) < 3:
            sys.exit("usage: gmail_cli.py feature <id> [<id>..]")
        cmd_feature(tok, sys.argv[2:])
    elif cmd == "unresolved":
        if len(sys.argv) < 3:
            sys.exit("usage: gmail_cli.py unresolved <id> [<id>..]")
        cmd_unresolved(tok, sys.argv[2:])
    elif cmd == "labels":
        cmd_labels(tok)
    else:
        sys.exit(f"unknown command: {cmd}")


if __name__ == "__main__":
    main()
