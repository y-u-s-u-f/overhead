# dmgbuild settings. Run through scripts/dmg.sh, which passes app and background.
import os.path

app = defines["app"]  # noqa: F821
app_name = os.path.basename(app)

format = "UDZO"
filesystem = "HFS+"
files = [app]
symlinks = {"Applications": "/Applications"}
hide_extension = [app_name]

background = defines["background"]  # noqa: F821
window_rect = ((200, 120), (660, 400))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
icon_size = 112
text_size = 13
icon_locations = {
    app_name: (170, 190),
    "Applications": (490, 190),
}
