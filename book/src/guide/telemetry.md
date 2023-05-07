# Telemetry

Orogene supports fully opt-in, anonymous telemetry in order to improve the
project and find issues that would otherwise not get reported, or lack enough
information to take action on.

## Configuration

You'll be prompted on first orogene run if you would like to enable telemetry.
It will not be enabled unless you explicitly say yes to the prompt. The
configuration will then be saved to your [global `oro.kdl` config
file](./configuration.md#the-orokdl-config-file), under `options { telemetry
<value>; }`. You can change your decision at any time by changing this
setting.

Telemetry is currently processed using [Sentry.io](https://sentry.io). If
you'd like to send telemetry information to your own Sentry organization, you
can do so with the `--sentry-dsn` option (or `sentry-dsn` in your `oro.kdl`
files, either global or per-project, or `oro_sentry_dsn` environment
variable).

## Privacy & PII

Orogene uses as many settings as possible in Sentry to make sure all possible
PII is scrubbed from telemetry events. Additionally, data is only retained for
90 days, including error reports, at which point it's automatically scrubbed
by Sentry. Unfortunately, this is not configurable.

Additionally, when errors happen, the `oro-debug-*.log` file may be uploaded
as an attachment to the error report. This may contain paths related to your
project, which may include the username, and the names of registries and
packages you may be using. It is recommended that you not opt in to telemetry
if this is unacceptable.

## Public Dashboard

In the interest of sharing, transparency, and helping Orogene's users, a
number of anonymous statistics collected from telemetry are made available [on
a public Grafana
dashboard](https://orogene.grafana.net/public-dashboards/f75247ab87e14eac9e11ad2034ae3f66?orgId=1).

Please note that dashboard queries may change at any time, but the general
intentions behind privacy/PII concerns will be maintained.
