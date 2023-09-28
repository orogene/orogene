# Authentication and Private Registries

Orogene supports logging in/out of both the main regisry, as well as
alternative/private registries. It supports three authentication methods:

## Using `oro login`

`oro login` supports configuring all authorization methods, and is able to
authenticate and log in and fetch a token for [Bearer Token](#bearer-token)
authorization.

When done, it will automatically add the relevant authorization credentials to
your global `oro.kdl`. If `--config <file>` is passed in, credentials will be
written to `<file>` instead. You can also pass in `--registry` to specify a
registry to log in to, and `--scope` to associate this registry with a
particular scope.

## Authorization Credentials

There's three possible method of providing authorization information when
interacting with a registry. Each of these can be configured by the `options >
auth` node in `oro.kdl`, with the node name being the registry the auth
information applies to. Additionally, `options > scoped-registries` will be
used to determine which registry auth should be picked for a particular
package.

For example:

```kdl
// oro.kdl
options {
    scoped-registries {
        "@mycompany" "https://my.company.registry.net"
    }
    auth {
        "https://registry.npmjs.org" token="deadbeef"
        "https://my.company.registry.net" username="myuser" password="mypassword"
    }
}
```

When making any requests to a registry, configured credentials will *always*
be automatically included in the `Authorization` header, encoded
appropriately. Authorization will also take into account scopes when fetching
or pushing individual packages and their metadata.

When package tarballs are hosted on a separate registry than the package's
configured registry (as determined by its scope or lack thereof),
authorization information will not be sent.


### Bearer Token

This is usually acquired through a login operation with the registry, and is
the preferred and more secure way of managing authorization.

Bearer token auth will be sent in the form of an HTTP header that looks like:

```
Authorization: Bearer deadbeefbadc0ffee
```

You can configure a bearer token using `oro login` by either invoking it
as-is, in which case you will be taken through an actual login flow with the
registry, or you can pass a `--token <token>` option directly to skip this, if
you already have a known token. You can also pass `--auth-type legacy` to log
in using classic command-line-prompt username/password instead of web-based
login. Unlike the main NPM CLI, an email is not collected, and a new account
cannot be create using `oro login`.

Given an invocation like `oro login --registry https://my.custom.registry.net
--scope @mycompany`, you will be taken to that registry's login page, and,
when done, your `oro.kdl` will look something like this:

```kdl
// oro.kdl
options {
    scoped-registries {
        "@mycompany" "https://my.custom.registry.net"
    }
    auth {
        "https://my.custom.registry.net" token="deadbeef"
    }
}
```

In NPM CLI terms, this maps to `:_authToken` and `:token`, which are synonyms.

### Basic Auth

You can provide a username and (optional) password to send to the configured
registry. This is not recommended if you can avoid it, since it involves
storing your auth information in plain text in an `oro.kdl` file, but is a
common practice for third-party registries.

Note that unlike the official NPM CLI, the password should _not_ be
base64-encoded, and should be stored in its original unencoded text.

You can use `oro login` to configure this authorization method, although no
authentication will happen: it will simply write it to your `oro.kdl`. To do
this, pass `--username <username>` and an optional `--password <password>`
when invoking `oro login`.

Basic auth will be sent in the form of an HTTP header that looks like:

```
Authorization: Basic ${toBase64(username + ":" + password)}
```

In NPM CLI terms, this maps to `:username` and `:_password`, and does not
require an `:email` equivalent to be set.

### Legacy Auth

Finally, you can provide what Orogene calls a "legacy" auth token, which is
essentially basic auth, and is used by certain tools to configure login
information. This token is not usually secure, since it's supposed to be
base64-encoded username and password information.

You can use `oro login` to configure this authorization method, although no
authentication will happen: it will simply write it to your `oro.kdl`. To do
this, pass `--legacy-token <token>` when invoking `oro login`.

Legacy auth will be sent as-is to the chosen registry:

```
Authorization: Basic deadbeefbadc0ffee
```

In NPM CLI terms, this maps to `:_auth`.
