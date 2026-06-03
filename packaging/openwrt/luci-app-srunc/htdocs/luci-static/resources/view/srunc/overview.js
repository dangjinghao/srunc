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
		'.srunc-actions{display:flex;flex-wrap:wrap;gap:.5em}'
	].join('\n'));
}

function renderStatus(s, profileName, keepalive) {
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

	if (online) {
		children.push(E('table', { 'class': 'srunc-kv' }, [
			E('tr', {}, [
				E('td', { 'class': 'srunc-k' }, _('User')),
				E('td', { 'class': 'srunc-v' }, s.username || '?')
			]),
			E('tr', {}, [
				E('td', { 'class': 'srunc-k' }, _('IP')),
				E('td', { 'class': 'srunc-v' }, s.ip || '?')
			])
		]));
	}

	return E('div', {}, children);
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
		var keepalive = (uci.get('srunc', 'default', 'keepalive') !== '0');
		var statusBox = E('div', {}, renderStatus(initial, profile, keepalive));

		// Refresh status every 5 s while the user is on the page.
		poll.add(function () {
			return callStatus('default').then(function (s) {
				dom.content(statusBox, renderStatus(s, profile, keepalive));
			}, function () { /* ignore transient errors */ });
		}, 5);

		return E([], [
			statusStyle(),
			E('h2', {}, _('srunc')),
			E('div', { 'class': 'cbi-section' }, [
				E('h3', {}, _('Status')),
				statusBox
			]),
			E('div', { 'class': 'cbi-section' }, [
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
			])
		]);
	},

	// Hide the default Save / Save & Apply / Reset bar — this view has no form.
	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
