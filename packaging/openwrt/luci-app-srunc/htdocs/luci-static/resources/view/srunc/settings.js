'use strict';
'require view';
'require form';
'require uci';
'require rpc';

// Built-in deployment profiles are owned by the Rust binary; fetch them at
// view load so the dropdown stays in sync with whatever the installed
// `srunc` supports (no JS-side duplicate list to keep current).
var callProfiles = rpc.declare({
	object: 'srunc',
	method: 'profiles',
	expect: { profiles: [] }
});

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

		s = m.section(form.TypedSection, 'srunc', _('Instances'));
		s.anonymous = false;
		s.addremove = true;
		s.addbtntitle = _('Add instance');

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

		o = s.taboption('basic', form.Value, 'password', _('Password'));
		o.password = true;
		o.rmempty = false;

		// ---- Advanced ----

		// host / ac_id are only meaningful when no profile is picked: the
		// profile wins over them in the resolution chain anyway, so hiding
		// the fields under a profile avoids the "I set host but it's ignored"
		// foot-gun. `depends('profile', '')` shows them only for Custom.
		o = s.taboption('advanced', form.Value, 'host', _('Portal host'),
			_('Override the auth host. Auto-discovered from the captive redirect if blank; falls back to <code>auth4/auth6.tsinghua.edu.cn</code>.'));
		o.optional = true;
		o.placeholder = 'auto';
		o.depends('profile', '');

		o = s.taboption('advanced', form.Value, 'ac_id', _('ac_id'),
			_('Override the access-controller id. Auto-discovered from the captive redirect if blank.'));
		o.optional = true;
		o.placeholder = 'auto';
		o.datatype = 'uinteger';
		o.depends('profile', '');

		o = s.taboption('advanced', form.Flag, 'ipv6', _('Use IPv6 portal'),
			_('Only affects the Tsinghua fallback default (<code>auth6</code> instead of <code>auth4</code>).'));
		o.default = '0';

		o = s.taboption('advanced', form.Flag, 'insecure', _('Use http://'),
			_('Use <code>http://</code> instead of <code>https://</code> for portal requests.'));
		o.default = '0';

		o = s.taboption('advanced', form.Value, 'timeout', _('HTTP timeout (seconds)'));
		o.datatype = 'uinteger';
		o.default = '2';
		o.placeholder = '2';

		// ---- Keepalive ----

		o = s.taboption('keepalive', form.Flag, 'keepalive', _('Enable keepalive'),
			_('Pass <code>-k</code> to srunc so it keeps the session alive.'));
		o.default = '1';

		o = s.taboption('keepalive', form.Flag, 'campus_only', _('In-campus keepalive probe'),
			_('Probe an in-campus URL instead of the public internet (useful where public traffic is billed).'));
		o.default = '0';

		o = s.taboption('keepalive', form.Value, 'interval', _('Probe interval (seconds)'));
		o.datatype = 'uinteger';
		o.default = '3';

		o = s.taboption('keepalive', form.Value, 'retry', _('Consecutive failures before exit'));
		o.datatype = 'uinteger';
		o.default = '2';

		return m.render();
	}
});
