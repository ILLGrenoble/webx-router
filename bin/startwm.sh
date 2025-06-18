# #!/bin/sh

# if [ -r /etc/default/locale ]; then
#   . /etc/default/locale
#   export LANG LANGUAGE
# fi

# startxfce4

#!/usr/bin/env bash

pre_start()
{
  if [ -r /etc/profile ]; then
    . /etc/profile
  fi
  if [ -r ~/.bash_profile ]; then
    . ~/.bash_profile
  else
    if [ -r ~/.bash_login ]; then
      . ~/.bash_login
    else
      if [ -r ~/.profile ]; then
        . ~/.profile
      fi
    fi
  fi

  xhost +si:localuser:$(id -un)

  return 0
}

post_start()
{
  if [ -r ~/.bash_logout ]; then
    . ~/.bash_logout
  fi
  return 0
}

get_xdg_session_startupcmd()
{
  # If DESKTOP_SESSION is set and valid then the STARTUP command will be taken from there
  # GDM exports environment variables XDG_CURRENT_DESKTOP and XDG_SESSION_DESKTOP.
  # This follows it.
  if [ -n "$1" ] && [ -d /usr/share/xsessions ] \
    && [ -f "/usr/share/xsessions/$1.desktop" ]; then
    STARTUP=$(grep ^Exec= "/usr/share/xsessions/$1.desktop")
    STARTUP=${STARTUP#Exec=*}
    XDG_CURRENT_DESKTOP=$(grep ^DesktopNames= "/usr/share/xsessions/$1.desktop")
    XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP#DesktopNames=*}
    XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP//;/:}
    export XDG_CURRENT_DESKTOP
    export XDG_SESSION_DESKTOP="$DESKTOP_SESSION"
  fi
}

#start the window manager
wm_start()
{
  if [ -r /etc/default/locale ]; then
    . /etc/default/locale
    export LANG LANGUAGE
  fi

  # debian
  if [ -r /etc/X11/Xsession ]; then
    pre_start

    # if you want to start preferred desktop environment,
    # add following line,
    #  [ -n "$XRDP_SESSION" ] && export DESKTOP_SESSION=<your preferred desktop>
    # in either of following file.
    # 1. ~/.profile
    # 2. create a file (any_filename.sh is OK) in /etc/profile.d
    # <your preferred desktop> shall be one of "ls -1 /usr/share/xsessions/|cut -d. -f1"
    # e.g.  [ -n "$XRDP_SESSION" ] && export DESKTOP_SESSION=ubuntu

    # STARTUP is the default startup command.
    # if $1 is empty and STARTUP was not set
    # /etc/X11/Xsession.d/50x11-common_determine-startup will fallback to
    # x-session-manager
    if [ -z "$STARTUP" ] && [ -n "$DESKTOP_SESSION" ]; then
      get_xdg_session_startupcmd "$DESKTOP_SESSION"
    fi

    #. /etc/X11/Xsession
    startxfce4

    post_start
    exit 0
  fi

  pre_start
  xterm
  post_start
}

wm_start

exit 1
