import argparse
import os
import subprocess
import sys
import tempfile

try:
    import simplejson as json
except ImportError:
    import json

SCRIPT_FILE = os.path.abspath(sys.argv[0])

def get_deploy_dir():
    return os.path.dirname(SCRIPT_FILE)

def get_outpost_dir():
    return os.path.join(get_deploy_dir(), '../..')

class Args(object):
    def __init__(self, ns, subparser):
        self.ns = ns
        self.subparser = subparser

    def __getattr__(self, key):
        val = getattr(self.ns, key)
        if val is None:
            opt = '--' + key.replace('_', '-')
            self.subparser.error('missing option %s' % opt)
        return val

    def get(self, key, default=None):
        val = getattr(self.ns, key)
        if val is None:
            return default
        return val

class TempCwd(object):
    def __init__(self):
        self.temp_dir = tempfile.TemporaryDirectory(suffix='.outpost-deploy')
        self.old_cwd = None

    def __enter__(self):
        self.old_cwd = os.getcwd()
        val = self.temp_dir.__enter__()
        os.chdir(val)
        return val

    def __exit__(self, exc_type, exc_value, traceback):
        drop_exc = self.temp_dir.__exit__(exc_type, exc_value, traceback)
        os.chdir(self.old_cwd)
        return drop_exc

def run(*args):
    subprocess.check_call(args)

def build_parser():
    ansible_args = argparse.ArgumentParser(add_help=False)
    ansible_args.add_argument('--server', metavar='ADDR',
            help='address of the backend server')
    ansible_args.add_argument('--ssh-port', metavar='PORT',
            default=22,
            help='port to use for SSH connections to the server')
    ansible_args.add_argument('--admin-user', metavar='NAME',
            default='admin',
            help='username to use for connecting to the server')
    ansible_args.add_argument('--admin-key', metavar='FILE',
            help='private key to use for connecting to the server')

    deploy_client_args = argparse.ArgumentParser(add_help=False)
    deploy_client_args.add_argument('--s3-path', metavar='URL',
            help='s3:// path to place client files')
    deploy_client_args.add_argument('--websocket-url', metavar='URL',
            help='''websocket URL to store in server.json
                (default: derive from --server address)''')
    deploy_client_args.add_argument('--downtime-message', metavar='MSG',
            help='message to include in the "server offline" dialog')

    deploy_server_args = argparse.ArgumentParser(add_help=False)
    deploy_server_args.add_argument('--daemon-user', metavar='NAME',
            default='outpost',
            help='username for running the Everfree Outpost daemon')
    deploy_server_args.add_argument('--daemon-public-key', metavar='FILE',
            help='''public key to install for the daemon account
                (default: use public key corresponding to --admin-key)''')

    parser = argparse.ArgumentParser(
            description='Deploy Everfree Outpost client and server components')

    subparsers = parser.add_subparsers(dest='command', metavar='COMMAND')

    sub_ansible = subparsers.add_parser('ansible',
            help='wrapper for running ansible commands',
            parents=[ansible_args])
    sub_ansible.add_argument('args', nargs='*',
            help='arguments to pass to ansible')

    sub_deploy_client = subparsers.add_parser('deploy-client',
            help='deploy client components to S3',
            parents=[deploy_client_args])

    sub_deploy_server = subparsers.add_parser('deploy-server',
            help='deploy server components using ansible',
            parents=[ansible_args, deploy_server_args])

    sub_deploy = subparsers.add_parser('deploy',
            help='deploy both server and client components',
            parents=[ansible_args, deploy_server_args, deploy_client_args])

    cmd_parsers = {
            'ansible': sub_ansible,
            'deploy-client': sub_deploy_client,
            'deploy-server': sub_deploy_server,
            'deploy': sub_deploy,
            }

    return (parser, cmd_parsers)


def main(argv):
    (parser, cmd_parsers) = build_parser()
    ns = parser.parse_args(argv)

    #from pprint import pprint
    #pprint(ns)

    if ns.command is None:
        parser.error('must specify a command')

    args = Args(ns, cmd_parsers[ns.command])

    with TempCwd():
        if ns.command == 'ansible':
            do_ansible(args)
        elif ns.command == 'deploy-server':
            do_deploy_server(args)
        elif ns.command == 'deploy-client':
            do_deploy_client(args)
        elif ns.command == 'deploy':
            do_deploy(args)
        else:
            parser.error('unsupported command "%s"' % ns.command)


def make_ansible_inventory(args):
    host_vars = {
            'ansible_ssh_host': args.server,
            'ansible_ssh_port': args.ssh_port,
            'ansible_ssh_user': args.admin_user,
            # Setting ansible_sudo here causes the 'synchronize' task to try to
            # 'sudo -u root' on the LOCAL machine.
            #'ansible_sudo': True,
            'ansible_connection': 'ssh',
            'ansible_ssh_private_key_file': args.admin_key,
            }
    vars_str = ' '.join('%s=%s' % (k, v) for k,v in host_vars.items())

    with open('inventory', 'w') as f:
        f.write('outpost %s\n' % vars_str)

def run_ansible(*args):
    run('ansible', 'outpost', '-M', get_deploy_dir(), '-i', 'inventory', *args)

def run_ansible_playbook(playbook, **kwargs):
    run('ansible-playbook', playbook, '-M', get_deploy_dir(), '-i', 'inventory', '-e', json.dumps(kwargs))

def get_public_key(private_key_file):
    # Run 'ssh-keygen -y' to convert the private key into a public key.
    pub_key = subprocess.check_output(['ssh-keygen', '-y', '-f', private_key_file])
    return pub_key.decode().strip()


def do_ansible(args):
    make_ansible_inventory(args)
    run_ansible(*args.args)

def do_deploy_server(args):
    make_ansible_inventory(args)
    playbook = os.path.join(get_deploy_dir(), 'playbook.yaml')

    daemon_key_file = args.get('daemon_public_key')
    if daemon_key_file is not None:
        with open(daemon_key_file) as f:
            daemon_key = f.read().strip()
    else:
        daemon_key = get_public_key(args.admin_key)

    run_ansible_playbook(playbook,
            daemon_user=args.daemon_user,
            daemon_public_key=daemon_key,
            dist_dir=os.path.join(get_outpost_dir(), 'dist'))

def do_deploy_client(args):
    s3_path = args.s3_path
    if not s3_path.endswith('/'):
        s3_path += '/'

    run('s3cmd', 'sync', '--exclude=server.json', '--delete-removed',
            os.path.join(get_outpost_dir(), 'dist', 'www') + '/',
            s3_path)

    with open('server.json', 'w') as f:
        downtime_message = args.get('downtime_message')
        if downtime_message is not None:
            msg = 'The server is currently offline.<br>' + downtime_message
            json.dump({ 'message': msg }, f)
        else:
            json.dump({ 'url': args.websocket_url }, f)
    run('s3cmd', 'put', 'server.json', s3_path + 'server.json')

def do_deploy(args):
    if getattr(args.ns, 'websocket_url', None) is None:
        args.ns.websocket_url = 'ws://%s:8888/ws' % args.server

    do_deploy_server(args)
    do_deploy_client(args)

if __name__ == '__main__':
    main(sys.argv[1:])
