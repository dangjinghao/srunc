'use strict';
'require view';
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

var callLogin = rpc.declare({
	object: 'srunc',
	method: 'login',
	expect: { '': {} }
});

var callLogout = rpc.declare({
	object: 'srunc',
	method: 'logout',
	expect: { '': {} }
});

var callRestart = rpc.declare({
	object: 'srunc',
	method: 'restart',
	expect: { '': {} }
});

var callProfiles = rpc.declare({
	object: 'srunc',
	method: 'profiles',
	expect: { profiles: [] }
});

// Resolve the configured `profile` id (default instance) into its human
// label. Returns null when no profile is selected (Custom / auto) — caller
// suppresses the badge entirely rather than showing an empty one.
function profileLabel(profiles) {
	var id = uci.get('srunc', 'default', 'profile') || '';
	if (!id) return null;
	for (var i = 0; i < profiles.length; i++) {
		if (profiles[i].id === id) return profiles[i].label;
	}
	return id;
}

// Resolve which portal host will actually be queried. Matches the daemon's
// resolution chain: explicit host (UCI) > profile.host > "auto-discovered".
// Returned object is { label, source } where source is one of
// 'explicit' | 'profile' | 'auto', so the UI can dim the auto case.
function resolvedPortal(profiles) {
	var explicit = (uci.get('srunc', 'default', 'host') || '').trim();
	if (explicit) return { label: explicit, source: 'explicit' };
	var pid = uci.get('srunc', 'default', 'profile') || '';
	if (pid) {
		for (var i = 0; i < profiles.length; i++) {
			if (profiles[i].id === pid && profiles[i].host) {
				return { label: profiles[i].host, source: 'profile' };
			}
		}
	}
	return { label: _('auto-discovered'), source: 'auto' };
}

// Scoped styling for the status card. Translucent backgrounds keep it legible
// on both light and dark LuCI themes.
function statusStyle() {
	return E('style', { 'type': 'text/css' }, [
		'.srunc-status{display:flex;align-items:center;gap:.55em;flex-wrap:wrap;margin:.3em 0 1em}',
		'.srunc-pill{display:inline-flex;align-items:center;gap:.45em;padding:.28em .9em;border-radius:999px;font-weight:600;line-height:1.5;border:1px solid transparent}',
		'.srunc-on{color:#1a8f3c;background:rgba(40,160,70,.14);border-color:rgba(40,160,70,.40)}',
		'.srunc-off{color:#8a8a8a;background:rgba(128,128,128,.14);border-color:rgba(128,128,128,.38)}',
		'.srunc-dot{width:.58em;height:.58em;border-radius:50%;background:currentColor;flex:0 0 auto}',
		'.srunc-badge{display:inline-flex;align-items:center;padding:.2em .6em;border-radius:6px;background:rgba(128,128,128,.15);font-size:.85em;font-weight:500}',
		'.srunc-kv{border-collapse:collapse;margin:.1em 0}',
		'.srunc-kv td{padding:.22em 1.4em .22em 0;vertical-align:top}',
		'.srunc-kv .srunc-k{opacity:.6;white-space:nowrap}',
		'.srunc-kv .srunc-v{font-weight:500}',
		'.srunc-v .srunc-auto{opacity:.6;font-style:italic;font-weight:400}',
		// Click-to-copy button: a small inline icon button next to copyable
		// values (user / ip / mac). Stays out of the way until you hover.
		'.srunc-copy{display:inline-flex;align-items:center;gap:.3em;margin-left:.5em;padding:0 .35em;border:1px solid transparent;border-radius:4px;background:transparent;color:inherit;cursor:pointer;font-size:.85em;opacity:.45;transition:opacity .15s,border-color .15s}',
		'.srunc-kv tr:hover .srunc-copy,.srunc-copy:focus,.srunc-copy:hover{opacity:1;border-color:rgba(128,128,128,.4)}',
		// Multi-device table.
		'.srunc-devices{margin:.6em 0 .2em;border-collapse:collapse;font-size:.9em;width:auto}',
		'.srunc-devices th,.srunc-devices td{padding:.3em .9em .3em 0;text-align:left;vertical-align:top}',
		'.srunc-devices th{opacity:.6;font-weight:500;border-bottom:1px solid rgba(128,128,128,.25)}',
		'.srunc-devices .srunc-current{font-weight:600;background:rgba(40,160,70,.10)}',
		'.srunc-devices-cap{margin:.7em 0 .25em;font-weight:600;opacity:.8}',
		// First-run "setup required" banner — non-blocking, dismissible
		// only by configuring credentials.
		'.srunc-setup{display:flex;align-items:center;gap:.8em;padding:.7em 1em;margin-bottom:.8em;border-radius:8px;background:rgba(240,180,40,.15);border:1px solid rgba(240,180,40,.45);color:inherit}',
		'.srunc-setup .srunc-setup-icon{font-size:1.4em;line-height:1}'
	].join('\n'));
}

// Format bytes as IEC units (KiB/MiB/GiB/...). Matches the CLI's format_bytes.
function fmtBytes(n) {
	if (typeof n !== 'number' || !isFinite(n)) return null;
	if (n < 1024) return n + ' B';
	var units = ['KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
	var v = n / 1024, u = 0;
	while (v >= 1024 && u + 1 < units.length) { v /= 1024; u += 1; }
	return v.toFixed(2) + ' ' + units[u];
}

// Coarse duration: "Xd Yh", "Yh Zm", or "Zm". Matches the CLI's format_seconds.
function fmtSeconds(s) {
	if (typeof s !== 'number' || !isFinite(s)) return null;
	var d = Math.floor(s / 86400);
	var h = Math.floor((s % 86400) / 3600);
	var m = Math.floor((s % 3600) / 60);
	if (d > 0) return d + 'd ' + h + 'h';
	if (h > 0) return h + 'h ' + m + 'm';
	return m + 'm';
}

// "just now" / "Xs ago" / ... — same buckets as the CLI's format_ago.
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

// Copy `text` to clipboard with a graceful fallback for non-secure-context
// LuCI installs (plain HTTP on the LAN). Promise resolves on success,
// rejects with an Error otherwise — caller toasts both outcomes.
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

// Small inline copy button. Returns null (not an empty element) when
// there's nothing to copy, so callers can spread it conditionally.
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

function kvRow(k, v, opts) {
	opts = opts || {};
	var children = [ v ];
	if (opts.copyable && typeof v === 'string' && v && v !== '?') {
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

// Render the multi-device sub-table from status.details.devices. Hidden
// when only one device is online — clutter for the common case. The row
// matching status.details.online_ip (or .ip if that's absent) is highlighted.
function renderDevices(devices, currentIp) {
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
	return E('div', {}, [
		E('div', { 'class': 'srunc-devices-cap' }, _('Online devices')),
		E('table', { 'class': 'srunc-devices' }, [head].concat(rows))
	]);
}

function renderStatus(s, profileName, keepalive, portal) {
	s = s || {};
	var online = !!s.online;

	var head = [
		E('span', { 'class': 'srunc-pill ' + (online ? 'srunc-on' : 'srunc-off') }, [
			E('span', { 'class': 'srunc-dot' }),
			online ? _('Online') : _('Offline')
		])
	];
	if (profileName)
		head.push(E('span', { 'class': 'srunc-badge' }, profileName));
	if (online && keepalive)
		head.push(E('span', { 'class': 'srunc-badge' }, _('keepalive')));

	var children = [ E('div', { 'class': 'srunc-status' }, head) ];

	// Portal row always visible — tells the user which host the daemon
	// resolved to (explicit override > profile host > auto-discovered).
	var rows = [];
	if (portal) {
		rows.push(kvRow(_('Portal'), portal.label, {
			auto: portal.source === 'auto',
			copyable: portal.source !== 'auto'
		}));
	}

	if (online) {
		rows.push(kvRow(_('User'), s.username || '?', { copyable: true }));
		rows.push(kvRow(_('IP'), s.ip || '?', { copyable: true }));

		// `details` is only populated for portals whose rad_user_info schema
		// the daemon has verified — currently THU. Other deployments get the
		// bare-bones rows above and skip the rest.
		var d = s.details;
		if (d) {
			if (d.plan)                rows.push(kvRow(_('Plan'),    d.plan));
			if (d.user_mac)            rows.push(kvRow(_('MAC'),     d.user_mac, { copyable: true }));
			if (typeof d.online_device_total === 'number')
				rows.push(kvRow(_('Devices'), String(d.online_device_total)));
			var used = fmtBytes(d.sum_bytes);
			if (used)                  rows.push(kvRow(_('Used'),    used));
			var time = fmtSeconds(d.sum_seconds);
			if (time)                  rows.push(kvRow(_('Time'),    time));
			var ka = fmtAgo(d.keepalive_time);
			if (ka)                    rows.push(kvRow(_('Last keepalive'), ka));
		}
	}

	if (rows.length > 0)
		children.push(E('table', { 'class': 'srunc-kv' }, rows));

	if (online && s.details) {
		var devTable = renderDevices(s.details.devices, s.details.online_ip || s.ip);
		if (devTable) children.push(devTable);
	}

	return E('div', {}, children);
}

// One-time non-blocking banner pointing first-time users at Settings.
// Suppressed once `username` is configured. Linked using LuCI's view
// navigation so it works regardless of admin URL prefix.
function setupBanner() {
	var hasUser = !!(uci.get('srunc', 'default', 'username') || '').trim();
	if (hasUser) return null;
	return E('div', { 'class': 'srunc-setup' }, [
		E('span', { 'class': 'srunc-setup-icon' }, '⚙️'),
		E('div', {}, [
			E('strong', {}, _('Setup required.')),
			' ',
			_('No username configured yet. '),
			E('a', { 'href': L.url('admin', 'services', 'srunc', 'settings') }, _('Open Settings')),
			' ',
			_('to enter your portal credentials.')
		])
	]);
}

function runAction(label, fn, ev) {
	var btn = ev.currentTarget;
	var orig = btn.innerHTML;
	btn.disabled = true;
	btn.innerHTML = '<em>' + _('Working…') + '</em>';
	return fn().then(function (r) {
		if (r && r.ok) {
			ui.addNotification(null, E('p', _('%s: ok').format(label)), 'success');
		} else {
			var err = (r && r.error) || _('failed');
			ui.addNotification(null, E('p', _('%s: %s').format(label, err)), 'danger');
		}
	}, function (e) {
		ui.addNotification(null, E('p', e.message || String(e)), 'danger');
	}).finally(function () {
		btn.disabled = false;
		btn.innerHTML = orig;
	});
}

return view.extend({
	load: function () {
		return Promise.all([
			callStatus('default').catch(function () { return {}; }),
			uci.load('srunc').catch(function () {}),
			callProfiles().catch(function () { return []; })
		]);
	},

	render: function (loaded) {
		var initial = (loaded && loaded[0]) || {};
		var profiles = (loaded && loaded[2]) || [];
		var profile = profileLabel(profiles);
		var portal = resolvedPortal(profiles);
		var keepalive = (uci.get('srunc', 'default', 'keepalive') !== '0');
		var statusBox = E('div', {}, renderStatus(initial, profile, keepalive, portal));

		// Refresh status every 5 s while the user is on the page.
		poll.add(function () {
			return callStatus('default').then(function (s) {
				dom.content(statusBox, renderStatus(s, profile, keepalive, portal));
			}, function () { /* ignore transient errors */ });
		}, 5);

		var banner = setupBanner();

		var page = [
			statusStyle(),
			E('h2', {}, _('srunc'))
		];
		if (banner) page.push(banner);
		page.push(E('div', { 'class': 'cbi-section' }, [
			E('h3', {}, _('Status')),
			statusBox
		]));
		page.push(E('div', { 'class': 'cbi-section' }, [
			E('h3', {}, _('Actions')),
			E('div', { 'class': 'srunc-actions' }, [
				E('button', {
					'class': 'btn cbi-button-action',
					'click': ui.createHandlerFn(this, runAction, _('login'), callLogin)
				}, [ _('Login now') ]),
				E('button', {
					'class': 'btn cbi-button-negative',
					'click': ui.createHandlerFn(this, runAction, _('logout'), callLogout)
				}, [ _('Logout') ]),
				E('button', {
					'class': 'btn cbi-button-neutral',
					'click': ui.createHandlerFn(this, runAction, _('restart'), callRestart)
				}, [ _('Restart service') ])
			])
		]));

		return E([], page);
	},

	// Hide the default Save / Save & Apply / Reset bar — this view has no form.
	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
