echo $HOME
echo $BUILD_TYPE
ls $HOME/.cargo/bin
python3 test/integration/main.py
$HOME/.cargo/bin/kcov
