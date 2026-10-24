# Privacy

Overhead has no server, no accounts, no analytics and no tracking. The developer never receives your location or anything else about you.

## What stays on your Mac

Your home location, radius and settings are saved in `~/Library/Application Support/Overhead`. Delete that folder to erase them.

## What leaves your Mac

Overhead sends your home location to these services:

- **[adsb.fi](https://adsb.fi)**, and **[adsb.lol](https://adsb.lol)** when adsb.fi doesn't respond, every 10 seconds, to find aircraft near home. **Share only an approximate location** is on by default. With it on, they get home rounded to about 1 km, not your exact address. You can change this on the location screen.
- **[OpenStreetMap Nominatim](https://osmfoundation.org/wiki/Privacy_Policy)**, only while you set home: to look up the address you type, or to show a name for the location you picked.

Like any website you visit, these services can see your IP address.

## Location access

If you choose **Use My Current Location**, macOS asks for permission. Overhead reads your location once, to set home, and never again. Every other time it uses the home you saved.
