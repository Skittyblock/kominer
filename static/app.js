const idInput = document.getElementById("user-id");
const passwordInput = document.getElementById("password");
const resultPassword = document.getElementById("result-password");
const toggleArchivedBtn = document.getElementById("toggle-archived");
const togglePasswordBtn = document.getElementById("toggle-result-password");

const LAST_ID_STORAGE_KEY = "kominer.lastUserId";
const ARCHIVED_ITEMS_STORAGE_KEY = "kominer.archivedVocabularyItems";

let eventSource = null;
let currentAuth = null;

let archivedItemIds = [];
let latestVocabularyItems = [];
let showResultPassword = false;
let showArchived = false;

onload();

function onload() {
	loadLastUserId();
	updateArchivedToggleLabel();
	updateResultPasswordDisplay();
	archivedItemIds = loadArchivedItemIds();

	// handle generate button in public mode
	const generateBtn = document.getElementById("generate-id");
	if (generateBtn) {
		generateBtn.addEventListener("click", () => {
			let id = "";
			for (let i = 0; i < 16; ++i) {
				id += Math.floor(Math.random() * 10);
			}
			idInput.value = id;
		});
	}

	togglePasswordBtn.addEventListener("click", () => {
		showResultPassword = !showResultPassword;
		updateResultPasswordDisplay();
	});

	toggleArchivedBtn.addEventListener("click", () => {
		showArchived = !showArchived;
		updateArchivedToggleLabel();
		renderVocabulary(latestVocabularyItems);
	});

	const form = document.getElementById("session-form");
	const statusEl = document.getElementById("status");
	const result = document.getElementById("result");
	const webdavUrl = document.getElementById("webdav-url");
	const username = document.getElementById("username");

	form.addEventListener("submit", async (event) => {
		event.preventDefault();

		const payload = {
			id: idInput.value.trim(),
			password: passwordInput.value,
		};

		const response = await fetch("/api/session", {
			method: "POST",
			headers: {
				"content-type": "application/json",
			},
			body: JSON.stringify(payload),
		});

		if (!response.ok) {
			let message = "failed to connect";

			try {
				const text = await response.text();
				if (text && text.trim()) {
					message = text.trim();
				}
			} catch (_) {}

			if (response.status === 429) {
				message = message || "session limit reached";
			}

			statusEl.textContent = message;
			result.hidden = false;
			return;
		}

		const data = await response.json();

		saveLastUserId(data.username);

		currentAuth = {
			username: data.username,
			password: data.password,
		};

		webdavUrl.textContent = `${window.location.origin}/dav/`;
		username.textContent = data.username;
		statusEl.textContent = "connected";
		form.hidden = true;
		result.hidden = false;

		showResultPassword = false;
		updateResultPasswordDisplay();

		await fetchVocabulary();

		if (eventSource) {
			eventSource.close();
		}

		eventSource = new EventSource(
			`/api/events?id=${encodeURIComponent(data.username)}&password=${encodeURIComponent(data.password)}`,
		);

		eventSource.addEventListener("file-updated", async (ev) => {
			const payload = JSON.parse(ev.data);
			statusEl.textContent = `file updated at ${payload.at}`;
			await fetchVocabulary();
		});

		eventSource.addEventListener("error", () => {
			statusEl.textContent = "disconnected";
		});
	});
}

function updateResultPasswordDisplay() {
	if (!currentAuth) {
		resultPassword.textContent = "";
		togglePasswordBtn.textContent = "show";
		togglePasswordBtn.disabled = true;
		return;
	}

	togglePasswordBtn.disabled = false;

	if (showResultPassword) {
		resultPassword.textContent = currentAuth.password;
		togglePasswordBtn.textContent = "hide";
	} else {
		resultPassword.textContent = "•".repeat(
			Math.max(8, currentAuth.password.length),
		);
		togglePasswordBtn.textContent = "show";
	}
}

function updateArchivedToggleLabel() {
	toggleArchivedBtn.textContent = showArchived
		? "hide archived"
		: "show archived";
}

// vocab

const vocabularySection = document.getElementById("vocabulary-section");
const vocabularySummary = document.getElementById("vocabulary-summary");
const vocabularyBody = document.getElementById("vocabulary-body");

async function fetchVocabulary() {
	if (!currentAuth) {
		return;
	}

	vocabularySection.hidden = false;
	vocabularySummary.textContent = "loading vocabulary...";

	try {
		const response = await fetch("/api/vocabulary", {
			headers: {
				Authorization: `Basic ${btoa(`${currentAuth.username}:${currentAuth.password}`)}`,
			},
		});

		if (response.status === 404) {
			vocabularySummary.textContent =
				"no vocabulary database uploaded yet.";
			vocabularyBody.innerHTML = "";
			return;
		}

		if (!response.ok) {
			vocabularySummary.textContent = "failed to load vocabulary.";
			vocabularyBody.innerHTML = "";
			return;
		}

		const data = await response.json();
		const items = Array.isArray(data.items) ? data.items : [];
		latestVocabularyItems = items;

		renderVocabulary(items);
	} catch (error) {
		console.error(error);
		vocabularySummary.textContent = "failed to load vocabulary.";
		vocabularyBody.innerHTML = "";
	}
}

function renderVocabulary(items) {
	vocabularyBody.innerHTML = "";

	const visibleItems = items.filter((item) => {
		const itemId = vocabularyItemId(item);
		return showArchived || !archivedItemIds.has(itemId);
	});

	const archivedCount = items.length - visibleItems.length;

	if (items.length === 0) {
		vocabularySummary.textContent = "no vocabulary entries found.";
		return;
	}

	vocabularySummary.textContent = showArchived
		? `${items.length} entr${items.length > 1 ? "ies" : "y"} total`
		: `${visibleItems.length} visible, ${archivedCount} archived`;

	if (visibleItems.length === 0) {
		const tr = document.createElement("tr");
		const td = document.createElement("td");
		td.colSpan = 5;
		td.textContent = showArchived
			? "no vocabulary entries found."
			: "no visible vocabulary entries.";
		tr.appendChild(td);
		vocabularyBody.appendChild(tr);
		return;
	}

	for (const item of visibleItems) {
		const tr = document.createElement("tr");
		const itemId = vocabularyItemId(item);
		const isArchived = archivedItemIds.has(itemId);

		if (isArchived) {
			tr.classList.add("archived");
		}

		appendArchiveCell(tr, item, isArchived);
		appendCell(tr, item.word);
		appendSentenceCell(
			tr,
			item.prev_context,
			item.highlight,
			item.next_context,
		);
		appendCell(tr, formatTimestamp(item.create_time));
		appendCell(tr, item.title);

		vocabularyBody.appendChild(tr);
	}
}

function appendCell(row, value) {
	const td = document.createElement("td");
	td.textContent = value ?? "";
	row.appendChild(td);
}

function appendSentenceCell(row, prevContext, highlight, nextContext) {
	const td = document.createElement("td");

	const prev = escapeHtml(prevContext ?? "");
	const hl = escapeHtml(highlight ?? "");
	const next = escapeHtml(nextContext ?? "");

	td.innerHTML = `${prev}<b>${hl}</b>${next}`;
	row.appendChild(td);
}

function appendArchiveCell(row, item, isArchived) {
	const td = document.createElement("td");
	const input = document.createElement("input");

	input.type = "checkbox";
	input.checked = isArchived;
	input.setAttribute("aria-label", "archive item");

	input.addEventListener("change", () => {
		const itemId = vocabularyItemId(item);

		if (input.checked) {
			archivedItemIds.add(itemId);
		} else {
			archivedItemIds.delete(itemId);
		}

		saveArchivedItemIds(archivedItemIds);
		renderVocabulary(latestVocabularyItems);
	});

	td.appendChild(input);
	row.appendChild(td);
}

function vocabularyItemId(item) {
	return item.word; // word is used as key in sqlite db
}

function escapeHtml(value) {
	return String(value)
		.replaceAll("&", "&amp;")
		.replaceAll("<", "&lt;")
		.replaceAll(">", "&gt;")
		.replaceAll('"', "&quot;")
		.replaceAll("'", "&#39;");
}

function formatTimestamp(value) {
	if (typeof value !== "number") {
		return "";
	}

	const date = new Date(value * 1000);
	if (Number.isNaN(date.getTime())) {
		return String(value);
	}

	return date.toLocaleString();
}

// storage

function loadLastUserId() {
	const savedId = localStorage.getItem(LAST_ID_STORAGE_KEY);
	if (savedId) {
		idInput.value = savedId;
	}
}

function saveLastUserId(id) {
	localStorage.setItem(LAST_ID_STORAGE_KEY, id);
}

function loadArchivedItemIds() {
	try {
		const raw = localStorage.getItem(ARCHIVED_ITEMS_STORAGE_KEY);
		if (!raw) {
			return new Set();
		}

		const parsed = JSON.parse(raw);
		if (!Array.isArray(parsed)) {
			return new Set();
		}

		return new Set(parsed);
	} catch (error) {
		console.error("failed to load archived items", error);
		return new Set();
	}
}

function saveArchivedItemIds(ids) {
	try {
		localStorage.setItem(
			ARCHIVED_ITEMS_STORAGE_KEY,
			JSON.stringify([...ids]),
		);
	} catch (error) {
		console.error("failed to save archived items", error);
	}
}
