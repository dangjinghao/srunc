'use strict';
'require view';
'require form';
'require rpc';
'require uci';
'require poll';
'require ui';
'require dom';

var callStatus = rpc.declare({
	object: 'srunc',
	method: 'status',
	params: [ 'section' ],
	expect: { '': {} }
});
var callLogout = rpc.declare({ object: 'srunc', method: 'logout',  expect: { '': {} } });
var callRestart = rpc.declare({ object: 'srunc', method: 'restart', expect: { '': {} } });
var callProfiles = rpc.declare({ object: 'srunc', method: 'profiles', expect: { profiles: [] } });
var callDiscover = rpc.declare({ object: 'srunc', method: 'discover', expect: { '': {} } });
var callServiceStatus = rpc.declare({ object: 'srunc', method: 'service_status', expect: { '': {} } });
var callVersion = rpc.declare({ object: 'srunc', method: 'version', expect: { '': {} } });
var callLog = rpc.declare({
	object: 'srunc',
	method: 'log',
	params: [ 'lines' ],
	expect: { log: '' }
});

// --- formatting helpers (mirror the CLI's print_status_human helpers) -----

function fmtBytes(n) {
	if (typeof n !== 'number' || !isFinite(n)) return null;
	if (n < 1024) return n + ' B';
	var units = ['KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
	var v = n / 1024, u = 0;
	while (v >= 1024 && u + 1 < units.length) { v /= 1024; u += 1; }
	return v.toFixed(2) + ' ' + units[u];
}
function fmtSeconds(s) {
	if (typeof s !== 'number' || !isFinite(s)) return null;
	var d = Math.floor(s / 86400);
	var h = Math.floor((s % 86400) / 3600);
	var m = Math.floor((s % 3600) / 60);
	if (d > 0) return d + 'd ' + h + 'h';
	if (h > 0) return h + 'h ' + m + 'm';
	return m + 'm';
}
function fmtAgo(ts) {
	if (typeof ts !== 'number' || !isFinite(ts)) return null;
	var now = Math.floor(Date.now() / 1000);
	if (ts > now) return String(ts);
	var d = now - ts;
	if (d < 5) return _('just now');
	if (d < 60) return d + 's ' + _('ago');
	if (d < 3600) return Math.floor(d / 60) + 'm ' + _('ago');
	if (d < 86400) return Math.floor(d / 3600) + 'h ' + _('ago');
	return Math.floor(d / 86400) + 'd ' + _('ago');
}
function fmtDate(ts) {
	if (typeof ts !== 'number' || !isFinite(ts) || ts <= 0) return null;
	return new Date(ts * 1000).toISOString().slice(0, 10);
}
function fmtUptime(sec) {
	if (typeof sec !== 'number' || !isFinite(sec) || sec < 0) return null;
	return fmtSeconds(sec);
}

function extractLastError(logText) {
	if (!logText) return null;
	var lines = String(logText).split('\n');
	for (var i = lines.length - 1; i >= 0; i--) {
		if (!/\berror:?\b/i.test(lines[i])) continue;
		var m = lines[i].match(/srunc\[\d+\]:\s*(.+)/);
		if (m) return m[1].trim().substring(0, 150);
	}
	return null;
}

// --- clipboard ------------------------------------------------------------

function copyText(text) {
	if (window.isSecureContext && navigator.clipboard && navigator.clipboard.writeText) {
		return navigator.clipboard.writeText(text);
	}
	return new Promise(function (resolve, reject) {
		try {
			var ta = document.createElement('textarea');
			ta.value = text;
			ta.setAttribute('readonly', '');
			ta.style.position = 'fixed';
			ta.style.top = '-1000px';
			document.body.appendChild(ta);
			ta.select();
			var ok = document.execCommand('copy');
			document.body.removeChild(ta);
			ok ? resolve() : reject(new Error('copy refused'));
		} catch (e) { reject(e); }
	});
}
function copyBtn(text) {
	if (!text) return null;
	var btn = E('button', {
		'type': 'button',
		'class': 'srunc-copy',
		'title': _('Copy "%s"').format(text),
		'aria-label': _('Copy "%s"').format(text)
	}, '⎘');
	btn.addEventListener('click', function (ev) {
		ev.preventDefault();
		copyText(text).then(function () {
			ui.addNotification(null, E('p', _('Copied: %s').format(text)), 'success');
		}, function () {
			ui.addNotification(null, E('p', _('Could not copy to clipboard.')), 'warning');
		});
	});
	return btn;
}

// --- resolved-portal lookup (config side; the daemon side now also
// surfaces resolved_host / resolved_ac_id in status JSON) -----------------

function profileLabel(profiles) {
	var id = uci.get('srunc', 'default', 'profile') || '';
	if (!id) return null;
	for (var i = 0; i < profiles.length; i++) {
		if (profiles[i].id === id) return profiles[i].label;
	}
	return id;
}

// --- KV rows --------------------------------------------------------------

function kvRow(k, v, opts) {
	opts = opts || {};
	if (v === null || v === undefined || v === '') return null;
	var children = [ v ];
	if (opts.copyable && typeof v === 'string' && v !== '?') {
		children.push(copyBtn(v));
	}
	if (opts.auto) {
		children = [ E('span', { 'class': 'srunc-auto' }, v) ];
	}
	return E('tr', {}, [
		E('td', { 'class': 'srunc-k' }, k),
		E('td', { 'class': 'srunc-v' }, children)
	]);
}

// --- styles ---------------------------------------------------------------

function dashboardStyle() {
	return E('style', { 'type': 'text/css' }, [
		// header / footer
		'.srunc-header{display:flex;align-items:baseline;justify-content:space-between;flex-wrap:wrap;gap:1em;margin:.2em 0 1em}',
		'.srunc-header h2{margin:0}',
		'.srunc-meta{opacity:.7;font-size:.85em;font-family:ui-monospace,Menlo,Consolas,monospace}',
		'.srunc-footer{margin-top:1.5em;padding-top:.6em;border-top:1px solid rgba(128,128,128,.25);opacity:.65;font-size:.8em;font-family:ui-monospace,Menlo,Consolas,monospace}',
		// status card (three-state)
		'.srunc-card{padding:1em 1.2em;border-radius:10px;border:1px solid transparent;margin:.3em 0 1em}',
		'.srunc-card-ok{background:rgba(40,160,70,.10);border-color:rgba(40,160,70,.35)}',
		'.srunc-card-warn{background:rgba(240,180,40,.12);border-color:rgba(240,180,40,.40)}',
		'.srunc-card-off{background:rgba(128,128,128,.08);border-color:rgba(128,128,128,.30)}',
		'.srunc-card-head{display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:.5em;margin-bottom:.6em}',
		'.srunc-card-main{display:flex;align-items:center;gap:.6em;font-size:1.15em;font-weight:600}',
		'.srunc-card-dot{width:.7em;height:.7em;border-radius:50%;flex:0 0 auto}',
		'.srunc-card-ok .srunc-card-dot{background:#1a8f3c}',
		'.srunc-card-warn .srunc-card-dot{background:#d4a017;animation:srunc-pulse 1.5s ease-in-out infinite}',
		'.srunc-card-off .srunc-card-dot{background:#8a8a8a}',
		'@keyframes srunc-pulse{0%,100%{opacity:1}50%{opacity:.35}}',
		'.srunc-card-badges{display:flex;gap:.4em;flex-wrap:wrap}',
		'.srunc-card-error{margin-top:.5em;padding:.4em .7em;border-radius:6px;background:rgba(240,180,40,.12);font-family:ui-monospace,Menlo,Consolas,monospace;font-size:.82em;opacity:.9;word-break:break-word}',
		'.srunc-card-actions{display:flex;flex-wrap:wrap;gap:.5em;margin-top:.8em;padding-top:.6em;border-top:1px solid rgba(128,128,128,.15)}',
		// badge
		'.srunc-badge{display:inline-flex;align-items:center;padding:.2em .6em;border-radius:6px;background:rgba(128,128,128,.15);font-size:.85em;font-weight:500}',
		// kv grid (two responsive columns)
		'.srunc-grid{display:grid;grid-template-columns:1fr 1fr;column-gap:2.5em;row-gap:.1em;margin:.2em 0 0}',
		'@media(max-width:768px){.srunc-grid{grid-template-columns:1fr}}',
		'.srunc-grid-col h4{margin:0 0 .3em 0;opacity:.7;font-size:.85em;font-weight:600;text-transform:uppercase;letter-spacing:.04em}',
		'.srunc-kv{border-collapse:collapse;width:100%;margin:0 0 .6em}',
		'.srunc-kv td{padding:.18em .8em .18em 0;vertical-align:top;font-size:.9em}',
		'.srunc-kv .srunc-k{opacity:.55;white-space:nowrap;width:6em}',
		'.srunc-kv .srunc-v{font-weight:500}',
		'.srunc-v .srunc-auto{opacity:.6;font-style:italic;font-weight:400}',
		// copy button (visible on row hover)
		'.srunc-copy{display:inline-flex;align-items:center;margin-left:.4em;padding:0 .35em;border:1px solid transparent;border-radius:4px;background:transparent;color:inherit;cursor:pointer;font-size:.85em;opacity:.35;transition:opacity .15s,border-color .15s}',
		'.srunc-kv tr:hover .srunc-copy,.srunc-copy:focus,.srunc-copy:hover{opacity:1;border-color:rgba(128,128,128,.4)}',
		// device sub-table
		'.srunc-devices{margin:.4em 0 1em;border-collapse:collapse;font-size:.85em;width:auto}',
		'.srunc-devices th,.srunc-devices td{padding:.25em .9em .25em 0;text-align:left;vertical-align:top}',
		'.srunc-devices th{opacity:.55;font-weight:500;border-bottom:1px solid rgba(128,128,128,.25)}',
		'.srunc-devices .srunc-current{font-weight:600;background:rgba(40,160,70,.10)}',
		// setup banner
		'.srunc-setup{display:flex;align-items:center;gap:.8em;padding:.7em 1em;margin-bottom:.8em;border-radius:8px;background:rgba(240,180,40,.15);border:1px solid rgba(240,180,40,.45)}',
		'.srunc-setup .srunc-setup-icon{font-size:1.4em;line-height:1}',
		// password show/hide toggle
		'.srunc-password-wrap{display:inline-flex;align-items:stretch;gap:.25em}',
		'.srunc-eye{padding:.1em .55em;line-height:1;cursor:pointer}',
		// advanced disclosure
		'details.srunc-advanced{margin:.4em 0 1em;border:1px solid rgba(128,128,128,.25);border-radius:6px;padding:.2em .8em}',
		'details.srunc-advanced[open]{padding-bottom:.6em}',
		'details.srunc-advanced > summary{cursor:pointer;padding:.45em 0;font-weight:600;opacity:.8;list-style:none}',
		'details.srunc-advanced > summary::-webkit-details-marker{display:none}',
		'details.srunc-advanced > summary::before{content:"▸ ";opacity:.6}',
		'details.srunc-advanced[open] > summary::before{content:"▾ "}'
	].join('\n'));
}

// --- header & footer ------------------------------------------------------

function renderHeader(version) {
	var bits = [];
	if (version && version.srunc) bits.push('srunc ' + version.srunc);
	if (version && version.luci_app) bits.push('luci-app ' + version.luci_app);
	var up = fmtUptime(version && version.uptime_sec);
	if (up) bits.push(_('daemon up') + ' ' + up);
	return E('div', { 'class': 'srunc-header' }, [
		E('h2', {}, _('srunc')),
		E('div', { 'class': 'srunc-meta' }, bits.join(' · ') || '')
	]);
}

function renderFooter(status, version, gateway) {
	var bits = [];
	if (version && version.srunc) bits.push('srunc ' + version.srunc);
	if (version && version.luci_app) bits.push('luci-app-srunc ' + version.luci_app);
	var up = fmtUptime(version && version.uptime_sec);
	if (up) bits.push(_('daemon up') + ' ' + up);
	if (gateway) bits.push(_('gateway') + ' ' + gateway);
	if (status && status.resolved_ac_id) bits.push('ac_id=' + status.resolved_ac_id);
	return E('div', { 'class': 'srunc-footer' }, bits.join(' · '));
}

// --- status card (three-state: authenticated / connecting / disconnected) ---

function renderStatusCard(status, serviceInfo, profiles, keepaliveEnabled, lastError, logoutBtn, restartBtn) {
	status = status || {};
	var d = status.details || {};
	var online = !!status.online;
	var running = !!(serviceInfo && serviceInfo.running);
	var state = online ? 'authenticated' : (running ? 'connecting' : 'disconnected');

	var cardClass = 'srunc-card ' + ({ authenticated: 'srunc-card-ok', connecting: 'srunc-card-warn', disconnected: 'srunc-card-off' })[state];
	var stateLabel = ({ authenticated: _('Authenticated'), connecting: _('Connecting…'), disconnected: _('Disconnected') })[state];

	var badges = [];
	var pLabel = profileLabel(profiles);
	if (pLabel) badges.push(E('span', { 'class': 'srunc-badge' }, pLabel));
	if (online && keepaliveEnabled) badges.push(E('span', { 'class': 'srunc-badge' }, _('keepalive')));

	var head = E('div', { 'class': 'srunc-card-head' }, [
		E('div', { 'class': 'srunc-card-main' }, [
			E('span', { 'class': 'srunc-card-dot' }),
			stateLabel
		]),
		badges.length ? E('div', { 'class': 'srunc-card-badges' }, badges) : null
	].filter(Boolean));

	var body;
	if (state === 'authenticated') {
		var sessionRows = [
			kvRow(_('User'), status.username || '?', { copyable: true }),
			kvRow(_('IPv4'), status.ip || d.online_ip || '?', { copyable: true }),
			d.devices ? kvRow(_('IPv6'), pickCurrentIp6(d.devices, status.ip || d.online_ip), { copyable: true }) : null,
			d.user_mac ? kvRow(_('MAC'), d.user_mac, { copyable: true }) : null,
			(typeof d.online_device_total === 'number') ? kvRow(_('Devices'), String(d.online_device_total)) : null,
			kvRow(_('Portal'), status.resolved_host || resolvedHostFromConfig(profiles) || '?', { copyable: !!status.resolved_host }),
			d.keepalive_time ? kvRow(_('Last kp'), fmtAgo(d.keepalive_time)) : null
		].filter(Boolean);

		var quotaRows = [];
		if (status.details) {
			quotaRows = [
				d.plan ? kvRow(_('Plan'), d.plan) : null,
				d.billing ? kvRow(_('Billing'), d.billing) : null,
				(typeof d.sum_bytes === 'number') ? kvRow(_('Used'), fmtBytes(d.sum_bytes)) : null,
				(typeof d.bytes_in === 'number') ? kvRow(_('↓ in'), fmtBytes(d.bytes_in)) : null,
				(typeof d.bytes_out === 'number') ? kvRow(_('↑ out'), fmtBytes(d.bytes_out)) : null,
				(typeof d.sum_seconds === 'number') ? kvRow(_('Time used'), fmtSeconds(d.sum_seconds)) : null,
				(typeof d.user_balance === 'number' && d.user_balance > 0) ? kvRow(_('Balance'), String(d.user_balance)) : null,
				d.add_time ? kvRow(_('Active since'), fmtDate(d.add_time)) : null
			].filter(Boolean);
		}

		var cols = [
			E('div', { 'class': 'srunc-grid-col' }, [
				E('h4', {}, _('Session')),
				E('table', { 'class': 'srunc-kv' }, sessionRows)
			])
		];
		if (quotaRows.length) {
			cols.push(E('div', { 'class': 'srunc-grid-col' }, [
				E('h4', {}, _('Plan & quota')),
				E('table', { 'class': 'srunc-kv' }, quotaRows)
			]));
		}
		body = E('div', { 'class': 'srunc-grid' }, cols);
	} else {
		var rows = [
			kvRow(_('Portal'), status.resolved_host || resolvedHostFromConfig(profiles) || '?', { copyable: !!status.resolved_host }),
			kvRow(_('ac_id'), status.resolved_ac_id || E('span', { 'class': 'srunc-auto' }, _('auto-discover')))
		].filter(Boolean);
		body = rows.length ? E('table', { 'class': 'srunc-kv' }, rows) : null;
	}

	var errorEl = (state === 'connecting' && lastError)
		? E('div', { 'class': 'srunc-card-error' }, lastError)
		: null;

	logoutBtn.disabled = !online;
	logoutBtn.style.display = online ? '' : 'none';
	var actionsEl = E('div', { 'class': 'srunc-card-actions' }, [ logoutBtn, restartBtn ]);

	return E('div', { 'class': cardClass }, [head, body, errorEl, actionsEl].filter(Boolean));
}

function pickCurrentIp6(devices, currentIp) {
	if (!devices || !devices.length) return null;
	var match = devices.find(function (dev) { return dev.ip === currentIp; });
	return (match && match.ip6) || null;
}

function resolvedHostFromConfig(profiles) {
	var explicit = (uci.get('srunc', 'default', 'host') || '').trim();
	if (explicit) return explicit;
	var pid = uci.get('srunc', 'default', 'profile') || '';
	if (pid) {
		for (var i = 0; i < profiles.length; i++) {
			if (profiles[i].id === pid && profiles[i].host) return profiles[i].host;
		}
	}
	return null;
}

function renderDevicesTable(devices, currentIp) {
	if (!devices || devices.length < 2) return null;
	var head = E('tr', {}, [
		E('th', {}, _('IP')),
		E('th', {}, _('IPv6')),
		E('th', {}, _('MAC')),
		E('th', {}, _('Session'))
	]);
	var rows = devices.map(function (d) {
		var mac = d.mac || d.class_name || '';
		var isCurrent = currentIp && d.ip === currentIp;
		return E('tr', { 'class': isCurrent ? 'srunc-current' : '' }, [
			E('td', {}, d.ip || ''),
			E('td', {}, d.ip6 || ''),
			E('td', {}, mac),
			E('td', {}, d.rad_online_id || '')
		]);
	});
	return E('table', { 'class': 'srunc-devices' }, [head].concat(rows));
}

// --- setup banner (shown when no username configured) ---------------------

function setupBanner() {
	var hasUser = !!(uci.get('srunc', 'default', 'username') || '').trim();
	if (hasUser) return null;
	return E('div', { 'class': 'srunc-setup' }, [
		E('span', { 'class': 'srunc-setup-icon' }, '⚙️'),
		E('div', {}, [
			E('strong', {}, _('Setup required.')),
			' ',
			_('Enter your portal credentials below and click Save & Apply.')
		])
	]);
}

// --- action verification (the 0.5.1 toast-accuracy fix) -------------------

function waitForOnline(expected, timeoutSec) {
	var deadline = Date.now() + timeoutSec * 1000;
	function step() {
		return callStatus('default').then(function (s) {
			if (!!(s && s.online) === !!expected) return s;
			if (Date.now() >= deadline) return null;
			return new Promise(function (r) { setTimeout(r, 1000); }).then(step);
		}, function () {
			if (Date.now() >= deadline) return null;
			return new Promise(function (r) { setTimeout(r, 1000); }).then(step);
		});
	}
	return step();
}

function runAction(opts, ev) {
	var btn = ev.currentTarget;
	var orig = btn.innerHTML;
	btn.disabled = true;
	btn.innerHTML = '<em>' + _('Working…') + '</em>';
	var timeoutSec = opts.timeoutSec || 12;
	return opts.fn().then(function (r) {
		if (!r || r.ok === false) {
			var err = (r && r.error) || _('dispatch failed');
			ui.addNotification(null, E('p', _('%s: %s').format(opts.label, err)), 'danger');
			return;
		}
		return waitForOnline(opts.expectedOnline, timeoutSec).then(function (st) {
			if (st === null) {
				ui.addNotification(null, E('p', opts.verifyFailure(timeoutSec)), 'warning');
			} else {
				ui.addNotification(null, E('p', opts.verifySuccess(st)), 'success');
			}
		});
	}, function (e) {
		ui.addNotification(null, E('p', e.message || String(e)), 'danger');
	}).finally(function () {
		btn.disabled = false;
		btn.innerHTML = orig;
	});
}

// --- form build ------------------------------------------------------------

// Marker class on each Advanced row so we can move them into <details>
// after form.render() returns the DOM.
var ADVANCED_NAMES = [
	'host', 'ac_id', '_discover', 'insecure', 'timeout', 'log_level',
	'keepalive', 'campus_only', 'interval', 'retry'
];

function configureForm(m, profiles) {
	var s = m.section(form.NamedSection, 'default', 'srunc', null);
	var o;

	o = s.option(form.Flag, 'enabled', _('Enabled'));
	o.default = '1';
	o.rmempty = false;

	o = s.option(form.ListValue, 'profile', _('Deployment'),
		_('Pick your srun deployment to use its built-in defaults. Choose <em>Custom</em> to fall through to captive-portal auto-discovery.'));
	o.value('', _('— Custom / auto —'));
	for (var i = 0; i < profiles.length; i++) {
		o.value(profiles[i].id, profiles[i].label);
	}
	o.default = '';
	o.rmempty = true;

	function previewWidget(depValue, lines) {
		var w = s.option(form.DummyValue, '_preview_' + (depValue || 'none'), _('Resolved'));
		w.depends('profile', depValue);
		w.rawhtml = true;
		w.cfgvalue = function () { return lines.join('<br>'); };
		return w;
	}
	previewWidget('', [
		'<em>' + _('Portal host: auto-discovered from the captive redirect') + '</em>',
		'<em>' + _('ac_id: auto-discovered from the captive redirect') + '</em>'
	]);
	for (var pi = 0; pi < profiles.length; pi++) {
		var p = profiles[pi];
		var lines = [
			p.host
				? _('Portal host: <strong>%s</strong>').format(p.host)
				: '<em>' + _('Portal host: auto-discovered (this deployment is label-only)') + '</em>',
			_('ac_id: <strong>auto-discovered</strong> per network segment')
		];
		previewWidget(p.id, lines);
	}

	o = s.option(form.Value, 'username', _('Username'));
	o.rmempty = false;
	o.placeholder = 'your-account';

	o = s.option(form.Value, 'password', _('Password'));
	o.password = true;
	o.rmempty = false;
	var origRender = o.renderWidget;
	o.renderWidget = function (section_id, option_index, cfgvalue) {
		var widget = origRender.apply(this, [section_id, option_index, cfgvalue]);
		var input = widget && widget.querySelector ? widget.querySelector('input[type=password]') : null;
		var hasNative = widget && widget.querySelector
			&& widget.querySelector('.cbi-button-toggle, .cbi-input-password-toggle');
		if (!input || hasNative) return widget;
		var btn = E('button', {
			'type': 'button',
			'class': 'cbi-button cbi-button-action srunc-eye',
			'title': _('Show/hide password'),
			'click': function (ev) {
				ev.preventDefault();
				if (input.type === 'password') { input.type = 'text'; ev.target.textContent = '🙈'; }
				else { input.type = 'password'; ev.target.textContent = '👁'; }
			}
		}, '👁');
		var wrap = E('span', { 'class': 'srunc-password-wrap' });
		input.parentNode.insertBefore(wrap, input);
		wrap.appendChild(input);
		wrap.appendChild(btn);
		return widget;
	};

	// ---- Advanced (will be DOM-wrapped in <details> after render) ----

	o = s.option(form.Value, 'host', _('Portal host'),
		_('Leave blank to use the host the selected <em>Deployment</em> profile carries, or to fall through to captive-portal auto-discovery if no profile is picked. Set a value here only to override both.'));
	o.optional = true;
	o.placeholder = _('blank = from profile / auto-discover');

	o = s.option(form.Value, 'ac_id', _('ac_id'),
		_('Leave blank to auto-discover the segment-correct ac_id from the captive redirect at every login. Set a value here only if you know your network segment\'s ac_id and want to skip the probe. <strong>Deployment profiles deliberately do NOT pin ac_id</strong> — it varies per dorm / building / VLAN.'));
	o.optional = true;
	o.placeholder = _('blank = auto-discover');
	o.datatype = 'uinteger';

	o = s.option(form.Button, '_discover', _('Discover from gateway'),
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

	o = s.option(form.Flag, 'insecure', _('Use http://'),
		_('Use plain <code>http://</code> instead of <code>https://</code> for portal requests. Leave off unless your portal cert is self-signed AND you trust the LAN you\'re on; otherwise authentication credentials transit unencrypted.'));
	o.default = '0';

	o = s.option(form.Value, 'timeout', _('HTTP timeout (seconds)'));
	o.datatype = 'uinteger';
	o.default = '2';
	o.placeholder = '2';

	o = s.option(form.ListValue, 'log_level', _('Log level'),
		_('Verbosity of the background daemon. Higher levels show more in the <em>Log</em> tab.'));
	o.value('error', _('Error'));
	o.value('warn',  _('Warning'));
	o.value('info',  _('Info'));
	o.value('debug', _('Debug'));
	o.default = 'info';

	o = s.option(form.Flag, 'keepalive', _('Enable keepalive'),
		_('Pass <code>-k</code> to srunc so it keeps the session alive.'));
	o.default = '1';

	o = s.option(form.Flag, 'campus_only', _('In-campus keepalive probe'),
		_('Probe an in-campus URL instead of the public internet (useful where public traffic is billed).'));
	o.default = '0';
	o.depends('keepalive', '1');

	o = s.option(form.Value, 'interval', _('Probe interval (seconds)'));
	o.datatype = 'uinteger';
	o.default = '3';
	o.depends('keepalive', '1');

	o = s.option(form.Value, 'retry', _('Consecutive failures before exit'));
	o.datatype = 'uinteger';
	o.default = '2';
	o.depends('keepalive', '1');
}

// Move every Advanced row into a <details> element.
function wrapAdvanced(formNode) {
	var rows = ADVANCED_NAMES.map(function (name) {
		return formNode.querySelector('[data-name="' + name + '"]');
	}).filter(Boolean);
	if (!rows.length) return;
	var anchor = rows[0];
	var details = E('details', { 'class': 'srunc-advanced' }, [
		E('summary', {}, _('Advanced'))
	]);
	anchor.parentNode.insertBefore(details, anchor);
	rows.forEach(function (row) { details.appendChild(row); });
}

// --- view -----------------------------------------------------------------

return view.extend({
	load: function () {
		return Promise.all([
			callStatus('default').catch(function () { return {}; }),
			callServiceStatus().catch(function () { return { running: false }; }),
			callVersion().catch(function () { return {}; }),
			uci.load('srunc').catch(function () {}),
			callProfiles().catch(function () { return []; })
		]);
	},

	render: function (loaded) {
		var initial = loaded[0] || {};
		var initialService = loaded[1] || { running: false };
		var versionInfo = loaded[2] || {};
		var profiles = loaded[4] || [];
		var self = this;

		var m = new form.Map('srunc');
		this.map = m;
		configureForm(m, profiles);

		return m.render().then(function (formNode) {
			wrapAdvanced(formNode);

			var keepalive = (uci.get('srunc', 'default', 'keepalive') !== '0');

			var logoutOpts = {
				label: _('logout'),
				fn: callLogout,
				expectedOnline: false,
				verifySuccess: function () { return _('Logged out.'); },
				verifyFailure: function (sec) {
					return _('Logout dispatched but the session is still online after %d s.').format(sec);
				}
			};
			var restartOpts = {
				label: _('restart'),
				fn: callRestart,
				expectedOnline: true,
				verifySuccess: function (st) {
					return st.username
						? _('Service restarted; logged in as %s.').format(st.username)
						: _('Service restarted.');
				},
				verifyFailure: function (sec) {
					return _('Service restarted but the daemon is still offline after %d s.').format(sec);
				}
			};
			var logoutBtn = E('button', {
				'class': 'btn cbi-button-negative',
				'click': ui.createHandlerFn(self, runAction, logoutOpts)
			}, [ _('Logout') ]);
			var restartBtn = E('button', {
				'class': 'btn cbi-button-neutral',
				'click': ui.createHandlerFn(self, runAction, restartOpts)
			}, [ _('Restart service') ]);

			var cardBox = E('div', {}, renderStatusCard(initial, initialService, profiles, keepalive, null, logoutBtn, restartBtn));
			var devicesBox = E('div', {},
				renderDevicesTable(initial.details && initial.details.devices,
					(initial.details && initial.details.online_ip) || initial.ip) || []);
			var footerBox = E('div', {}, renderFooter(initial, versionInfo, null));

			poll.add(function () {
				return Promise.all([
					callStatus('default').catch(function () { return {}; }),
					callServiceStatus().catch(function () { return { running: false }; })
				]).then(function (results) {
					var s = results[0];
					var svc = results[1];
					var online = !!(s && s.online);
					var running = !!(svc && svc.running);
					var errorP = (!online && running)
						? callLog(10).then(extractLastError).catch(function () { return null; })
						: Promise.resolve(null);
					return errorP.then(function (lastError) {
						dom.content(cardBox, renderStatusCard(s, svc, profiles, keepalive, lastError, logoutBtn, restartBtn));
						var t = renderDevicesTable(s.details && s.details.devices,
							(s.details && s.details.online_ip) || s.ip);
						dom.content(devicesBox, t || []);
						dom.content(footerBox, renderFooter(s, versionInfo, null));
					});
				});
			}, 5);

			var banner = setupBanner();
			var page = [
				dashboardStyle(),
				renderHeader(versionInfo)
			];
			if (banner) page.push(banner);
			page.push(cardBox, devicesBox, formNode, footerBox);
			return E([], page);
		});
	},

	handleSaveApply: function (ev, mode) {
		return this.map.save().then(function () {
			return ui.changes.apply(mode == '0');
		});
	}
});
