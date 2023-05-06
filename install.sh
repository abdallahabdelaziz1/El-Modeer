#!/bin/bash

# install

# --git https://github.com/abdallahabdelaziz1/El-Modeer

if cargo install --path .; then

    echo "El-modeer is installed Successfully!"

    # define the alias and command
    alias_name="deer"
    alias_command="el-modeer"

    # append the alias to ~/.bash_aliases
    echo "alias $alias_name='$alias_command'" >> ~/.bash_aliases

    # update the ~/.bashrc file to source ~/.bash_aliases
    if ! grep -q "~/.bash_aliases" ~/.bashrc; then
        echo "if [ -f ~/.bash_aliases ]; then" >> ~/.bashrc
        echo "    . ~/.bash_aliases" >> ~/.bashrc
        echo "fi" >> ~/.bashrc
    fi

    # reload the bash configuration
    source ~/.bashrc

    exec bash

else
    echo "An error occured"
fi