# kominer

a tool to mine sentences from [koreader](https://koreader.rocks/) vocabulary builder using browser extensions like [yomitan](https://yomitan.wiki/).

a public instance of this may be running on [kominer.skitty.xyz](https://kominer.skitty.xyz), if i haven't taken it down.
you can try it out there, but it works better if you have a way to self-host it yourself.

## prerequisites

- [yomitan](https://yomitan.wiki/) (or equivalent) configured with ankiconnect
- a device with [koreader](https://koreader.rocks/) installed that can connect to the internet

## usage

this tool runs a webdav server, which the user can configure their koreader vocabulary builder to sync to.
- in public mode: users can create their own credentials for the webdav server, which will provision them a temporary storage location only they can access.
- in private mode: you provide a single username and password that will always be accessible without starting a session.
when koreader syncs to the server, the client displays all vocabulary builder entries with context, and the user may mark them as archived after adding them to anki.

step by step:
1. open the web page served by the program in a browser.
2. generate a 16 digit id and optionally enter a password, or enter the `KOMINER_USER` and `KOMINER_PASSWORD` env variables you provided.
	- in public mode, if someone else is currently using the id, you won't be able to access it unless you use the same password.
3. add a webdav cloud sync server in koreader, using the displayed details.
4. after pressing the sync button in the vocabulary builder page in koreader, the webpage should update.
	- there's [a koreader bug](https://github.com/koreader/koreader/issues/14002) where the sync'd db won't be updated unless you quit the app and restart it, so keep that in mind.
5. hover over bolded words in the sentence and add to your anki deck using yomitan.
6. mark the word as achived, so it will be hidden the next time you sync.

note that in public mode you can only connect to the server while the session is active, which is only when you go to the site and press start.
the intended use case is to primarily read and mine offline, then connect once (per day/week/month) and sync everything.
if you self host, the webdav server will always be accessible with the static credentials you provide, so you can sync at any time.
