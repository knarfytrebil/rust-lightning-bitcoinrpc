echo $HOME
ls $HOME/.cargo/bin
RUNNING_ENV=docker python3 test/integration/main.py
bash <(curl -s https://codecov.io/bash) -s $HOME/coverage 
