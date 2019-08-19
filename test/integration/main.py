import os, datetime, time
import subprocess

# Print Messages
def get_now():
    now = datetime.datetime.now()
    return now.strftime('%H:%M:%S')

def print_info(message):
    print("{} \x1b[1;34m[ INFO]\x1b[0m {} ... ".format(get_now(), message))

def print_warn(message):
    print("{} \x1b[1;33m[ WARN]\x1b[0m {} ... ".format(get_now(), message))

def print_pass(message):
    print("{} \x1b[1;32m[ PASS]\x1b[0m {} ... ".format(get_now(), message))

def print_error(message):
    print("{} \x1b[1;31m[ERROR]\x1b[0m {} ... ".format(get_now(), message))

def get_env(test_version):
    working_dir = os.getcwd() + "/../../"
    server_dir = working_dir + "server/"
    client_dir = working_dir + "cli/"
    conf_dir = working_dir + "test/conf/"

    environment = {
        "working_dir": working_dir,
        "server": { 
            "bin": "rustbolt",
            "root": server_dir,
            "test": server_dir + 'target/' + test_version + '/'
        },
        "cli": {
            "bin": "rbcli",
            "root": client_dir,
            "test": client_dir + 'target/' + test_version + '/'
        },
        "conf": {
            "root" : conf_dir,
            "server": { 
                "dir": conf_dir + 'server/',
                "ln": "ln.conf.toml",
                "node": "node.conf.toml"
            }
        }
    }
    return environment

def build(project, version, env):
    print_info("building {} version: {}".format(project, version))
    os.chdir(env[project]["root"])

    if subprocess.run(["cargo", "build"]).returncode != 0:
        return print_error("build Error")
    print_pass("build success, {} is ready".format(project))
    return env[project]["test"]

def run_server(server_id, build_dir, version, env):
    server_bin =  build_dir + env["server"]["bin"] 

    # Copy configuration files
    print_info("copying configuration files")
    conf = "{}{}/".format(env["conf"]["server"]["dir"],server_id)
    subprocess.run(["cp", "-r", conf, "{}{}".format(build_dir, server_id)])

    data = build_dir + "ln/data_{}/".format(server_id)
    print_info("preparing local storage files: {}".format(data))

    # Create data storage folder 
    subprocess.run(["mkdir", "-p", data])
    os.chdir(build_dir)

    # Run server 
    print_info("run: { " + server_bin + " }")
    server = subprocess.Popen([
        server_bin, 
        "{}{}/{}".format(build_dir, server_id, env["conf"]["server"]["ln"]),
        "{}{}/{}".format(build_dir, server_id, env["conf"]["server"]["node"]),
    ])

    if server.returncode != 0 and server.returncode != None:
        return print_error("server run error") 

    print_pass("servre running:[PID]:{} [SERVER_ID]:{}".format(
        str(server.pid),
        server_id
    ))

    return server

def run_cli(build_dir, env, cmd):
    cli_bin =  build_dir + env["cli"]["bin"] 
    cli = subprocess.run([cli_bin, ] + cmd)

def main():
    env = get_env("debug")
    TEST_OVER = False

    # Build Lightning Server
    server_build_dir = build("server", "debug", env)
    
    # Build Cli
    cli_build_dir = build("cli", "debug", env)

    # Run Server
    s1 = run_server(1, server_build_dir, "debug", env)
    s2 = run_server(2, server_build_dir, "debug", env)

    print_info("waiting for server to stablize, counting for 5 secs")
    time.sleep(5)

    
    print_info("rbcli info -an")
    run_cli(cli_build_dir, env, ["info", "-an"])
    print_info("rbcli -n 127.0.0.1:8124 info -an")
    run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "info", "-an"])
    print_pass("get info success")

    s1.kill()
    s2.kill()
     
main()
