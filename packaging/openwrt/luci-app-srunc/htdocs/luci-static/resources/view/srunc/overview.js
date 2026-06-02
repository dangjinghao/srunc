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
// suppresses the row entirely rather than showing an empty separator.
function profileLabel(profiles) {
	var id = uci.get('srunc', 'default', 'profile') || '';
	if (!id) return null;
	for (var i = 0; i < profiles.length; i++) {
		if (profiles[i].id === id) return profiles[i].label;
	}
	return id;
}

function renderStatus(s, profileName) {
	s = s || {};
	var online = !!s.online;
	var dot = online ? '●' : '○';
	var label = online ? _('Online') : _('Offline');
	var color = online ? '#2a8c2a' : '#a33';

	var headline = dot + ' ' + label;
	if (profileName) headline += '  —  ' + profileName;

	var children = [
		E('div', {
			'style': 'font-size: 1.4em; font-weight: 600; color: ' + color + ';'
		}, headline)
	];

	if (online) {
		children.push(E('table', { 'class': 'table' }, [
			E('tr', { 'class': 'tr' }, [
				E('td', { 'class': 'td left', 'width': '120' }, _('User')),
				E('td', { 'class': 'td left' }, s.username || '?')
			]),
			E('tr', { 'class': 'tr' }, [
				E('td', { 'class': 'td left' }, _('IP')),
				E('td', { 'class': 'td left' }, s.ip || '?')
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
		var statusBox = E('div', {}, renderStatus(initial, profile));

		// Refresh status every 5 s while the user is on the page.
		poll.add(function () {
			return callStatus('default').then(function (s) {
				dom.content(statusBox, renderStatus(s, profile));
			}, function () { /* ignore transient errors */ });
		}, 5);

		return E([], [
			E('h2', {}, _('srunc')),
			E('div', { 'class': 'cbi-section' }, [
				E('h3', {}, _('Status')),
				statusBox
			]),
			E('div', { 'class': 'cbi-section' }, [
				E('h3', {}, _('Actions')),
				// Plain flex row — NOT cbi-page-actions, which flex-grows the
				// middle child (it's meant for the Save & Apply bar at the
				// bottom of forms). We want three same-sized inline buttons.
				E('div', {
					'style': 'display: flex; flex-wrap: wrap; gap: 0.5em;'
				}, [
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
