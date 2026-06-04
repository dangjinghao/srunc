'use strict';
'require view';
'require form';
'require uci';
'require rpc';
'require ui';
'require dom';

// Built-in deployment profiles are owned by the Rust binary; fetch them at
// view load so the dropdown stays in sync with whatever the installed
// `srunc` supports (no JS-side duplicate list to keep current).
var callProfiles = rpc.declare({
	object: 'srunc',
	method: 'profiles',
	expect: { profiles: [] }
});

// Probe the default gateway for the captive-portal redirect; returns
// {gateway, host, ac_id} on success. Backs the Discover button below.
var callDiscover = rpc.declare({
	object: 'srunc',
	method: 'discover',
	expect: { '': {} }
});

// Scoped styles for the show/hide password button so it sits flush with the
// input without distorting LuCI's grid layout.
function settingsStyle() {
	return E('style', { 'type': 'text/css' }, [
		'.srunc-password-wrap{display:inline-flex;align-items:stretch;gap:.25em}',
		'.srunc-eye{padding:.1em .55em;line-height:1;cursor:pointer}',
		'.srunc-discover-row{margin:.4em 0 .8em 0}',
		'.srunc-discover-row .cbi-button{margin-right:.6em}',
		'.srunc-discover-hint{opacity:.7;font-size:.9em}'
	].join('\n'));
}

return view.extend({
	load: function () {
		return Promise.all([
			uci.load('srunc'),
			callProfiles().catch(function () { return []; })
		]);
	},

	render: function (loaded) {
		var profiles = (loaded && loaded[1]) || [];
		var m, s, o;

		m = new form.Map('srunc',
			_('srunc — srun captive-portal auth'),
			_('Set username and password; the portal host and ac_id are auto-discovered from the captive redirect, so the override fields can stay blank. Pick a deployment profile below to use built-in defaults instead.'));

		s = m.section(form.NamedSection, 'default', 'srunc', _('Settings'));

		s.tab('basic',     _('Basic'));
		s.tab('advanced',  _('Advanced'));
		s.tab('keepalive', _('Keepalive'));

		// ---- Basic ----

		o = s.taboption('basic', form.Flag, 'enabled', _('Enabled'));
		o.default = '1';
		o.rmempty = false;

		o = s.taboption('basic', form.ListValue, 'profile', _('Deployment'),
			_('Pick your srun deployment to use built-in portal defaults (host + ac_id). Choose <em>Custom</em> to enter them manually under <em>Advanced</em> or to rely on captive-portal auto-discovery.'));
		o.value('', _('— Custom / auto —'));
		for (var i = 0; i < profiles.length; i++) {
			o.value(profiles[i].id, profiles[i].label);
		}
		o.default = '';
		o.rmempty = true;

		o = s.taboption('basic', form.Value, 'username', _('Username'));
		o.rmempty = false;
		o.placeholder = 'your-account';

		// Password field with a show/hide toggle. We post-process the
		// rendered widget rather than subclassing form.Value, so this
		// stays compatible with LuCI's built-in password handling.
		o = s.taboption('basic', form.Value, 'password', _('Password'));
		o.password = true;
		o.rmempty = false;
		var origRenderWidget = o.renderWidget;
		o.renderWidget = function (section_id, option_index, cfgvalue) {
			var widget = origRenderWidget.apply(this, [section_id, option_index, cfgvalue]);
			var input = widget && widget.querySelector
				? widget.querySelector('input[type=password]')
				: null;
			// Some LuCI builds already ship a reveal toggle. Detect the
			// telltale class to avoid stacking two buttons.
			var hasNative = widget && widget.querySelector
				&& widget.querySelector('.cbi-button-toggle, .cbi-input-password-toggle');
			if (!input || hasNative) return widget;

			var btn = E('button', {
				'type': 'button',
				'class': 'cbi-button cbi-button-action srunc-eye',
				'title': _('Show/hide password'),
				'click': function (ev) {
					ev.preventDefault();
					if (input.type === 'password') {
						input.type = 'text';
						ev.target.textContent = '🙈';
					} else {
						input.type = 'password';
						ev.target.textContent = '👁';
					}
				}
			}, '👁');

			var wrap = E('span', { 'class': 'srunc-password-wrap' });
			input.parentNode.insertBefore(wrap, input);
			wrap.appendChild(input);
			wrap.appendChild(btn);
			return widget;
		};

		// ---- Advanced ----

		// host / ac_id are always editable: some profiles only carry a
		// label (verification pending) and need these to actually log in.
		// When a profile DOES pin a value, the explicit field still wins
		// (resolution chain: explicit → profile → discovery).
		o = s.taboption('advanced', form.Value, 'host', _('Portal host'),
			_('Override the auth host. Picked profile wins over auto-discovery; this field wins over both.'));
		o.optional = true;
		o.placeholder = 'auto';

		o = s.taboption('advanced', form.Value, 'ac_id', _('ac_id'),
			_('Override the access-controller id. Picked profile wins over auto-discovery; this field wins over both.'));
		o.optional = true;
		o.placeholder = 'auto';
		o.datatype = 'uinteger';

		// "Discover from gateway" — Button widget on the Advanced tab,
		// next to the host / ac_id fields it prefills. Click probes the
		// gateway via the rpcd `discover` method and writes the result
		// back into the host / ac_id form fields so the user just has
		// to click Save & Apply.
		o = s.taboption('advanced', form.Button, '_discover', _('Discover from gateway'),
			_('Probe the default gateway for the captive-portal redirect and prefill <em>Portal host</em> + <em>ac_id</em> above.'));
		o.inputtitle = _('Discover');
		o.inputstyle = 'apply';
		o.onclick = function (ev, section_id) {
			var btn = ev.currentTarget;
			var orig = btn.value;
			btn.disabled = true;
			btn.value = _('Probing…');
			return callDiscover().then(function (r) {
				if (r && r.host) {
					var hostOpt = s.getOption('host');
					var acOpt = s.getOption('ac_id');
					if (hostOpt) hostOpt.getUIElement(section_id).setValue(r.host);
					if (acOpt && r.ac_id) acOpt.getUIElement(section_id).setValue(r.ac_id);
					ui.addNotification(null,
						E('p', _('Discovered host %s, ac_id %s').format(r.host, r.ac_id || '?')),
						'success');
				} else {
					var msg = (r && r.error) || _('no captive redirect found');
					ui.addNotification(null, E('p', _('Discover failed: %s').format(msg)), 'warning');
				}
			}, function (e) {
				ui.addNotification(null, E('p', e.message || String(e)), 'danger');
			}).finally(function () {
				btn.disabled = false;
				btn.value = orig;
			});
		};

		o = s.taboption('advanced', form.Flag, 'insecure', _('Use http://'),
			_('Use plain <code>http://</code> instead of <code>https://</code> for portal requests. \
Leave off unless your portal cert is self-signed AND you trust the LAN you\'re on; \
otherwise authentication credentials transit unencrypted.'));
		o.default = '0';

		o = s.taboption('advanced', form.Value, 'timeout', _('HTTP timeout (seconds)'));
		o.datatype = 'uinteger';
		o.default = '2';
		o.placeholder = '2';

		o = s.taboption('advanced', form.ListValue, 'log_level', _('Log level'),
			_('Verbosity of the background daemon. Higher levels show more in the <em>Log</em> tab.'));
		o.value('error', _('Error'));
		o.value('warn',  _('Warning'));
		o.value('info',  _('Info'));
		o.value('debug', _('Debug'));
		o.default = 'info';

		// ---- Keepalive ----

		o = s.taboption('keepalive', form.Flag, 'keepalive', _('Enable keepalive'),
			_('Pass <code>-k</code> to srunc so it keeps the session alive.'));
		o.default = '1';

		// Sub-options hide when keepalive is off — `.depends('keepalive', '1')`
		// has LuCI handle the show/hide automatically. The defaults stay set
		// in UCI either way so toggling keepalive back on keeps the values.
		o = s.taboption('keepalive', form.Flag, 'campus_only', _('In-campus keepalive probe'),
			_('Probe an in-campus URL instead of the public internet (useful where public traffic is billed).'));
		o.default = '0';
		o.depends('keepalive', '1');

		o = s.taboption('keepalive', form.Value, 'interval', _('Probe interval (seconds)'));
		o.datatype = 'uinteger';
		o.default = '3';
		o.depends('keepalive', '1');

		o = s.taboption('keepalive', form.Value, 'retry', _('Consecutive failures before exit'));
		o.datatype = 'uinteger';
		o.default = '2';
		o.depends('keepalive', '1');

		return Promise.resolve(m.render()).then(function (rendered) {
			// Prepend our scoped styles so the password wrapper / discover row
			// pick them up without bleeding into the rest of LuCI.
			return E([], [ settingsStyle(), rendered ]);
		});
	}
});
