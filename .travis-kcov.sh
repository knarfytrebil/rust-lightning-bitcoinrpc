echo $HOME
echo $BUILD_TYPE
ls $HOME/.cargo/bin
RUNNING_ENV=docker python3 test/integration/main.py
