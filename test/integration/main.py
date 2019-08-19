import os, datetime, time
import subprocess, json

# Print Messages
def get_now():
    now = datetime.datetime.utcnow()
    return now.strftime('%H:%M:%S')

def print_info(message):
    print("{} \x1b[1;34m[ INFO]\x1b[0m {} ... ".format(get_now(), message))

def print_exec(message):
    print("{} \x1b[1;33m[ EXEC]\x1b[0m {}".format(get_now(), message))

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
            "test": "{}target/{}/".format(server_dir,test_version) },
        "cli": {
            "bin": "rbcli",
            "root": client_dir,
            "test": "{}target/{}/".format(client_dir,test_version)
        },
        "conf": {
            "root" : conf_dir,
            "server": { 
                "dir": "{}server/".format(conf_dir),
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
    print_exec(">>> rbcli {}".format(" ".join(cmd)))
    cli_bin =  build_dir + env["cli"]["bin"] 
    return json.loads(subprocess.check_output([cli_bin, "-j"] + cmd).decode('ascii'))

def main():
    env = get_env("debug")

    # Build Lightning Server
    server_build_dir = build("server", "debug", env)
    
    # Build Cli
    cli_build_dir = build("cli", "debug", env)

    # Run Server
    s1 = run_server(1, server_build_dir, "debug", env)
    s2 = run_server(2, server_build_dir, "debug", env)

    print_info("waiting for server to stablize, counting for 5 secs")
    time.sleep(5)

    # Info
    r0 = run_cli(cli_build_dir, env, ["info", "-a"])
    print_pass("got node #1 addresses: {}".format(r0))
   
    r1 = run_cli(cli_build_dir, env, ["info", "-n"])
    print_pass("got node #1 public key: {}".format(r1["node_id"]))

    r2 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "info", "-n"])
    print_pass("got node #2 public key: {}".format(r2["node_id"]))

    # Peer 
    r3 = run_cli(cli_build_dir, env, ["peer", "-c", "{}@{}:{}".format(r2["node_id"], "127.0.0.1", "9736")])
    print_pass("got connection: {}".format(r3))

    time.sleep(5)
    r4 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "peer", "-l"])
    print_pass("got node #2 peers: {}".format(r4))

    # Channel 
    r5 = run_cli(cli_build_dir, env, ["channel", "-c", r1["node_id"], "100000", "5000"])
    print_pass("got channel: {}".format(r5))

    r6 = run_cli(cli_build_dir, env, ["-n", "127.0.0.1:8124", "channel", "-l"])
    print_pass("got channel list: {}".format(r6))

    s1.kill()
    s2.kill()
     
main()
