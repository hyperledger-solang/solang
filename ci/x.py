import os

for root, dirs, files in os.walk('C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\'):
    for name in files:
        if name.endswith(".bat"):
            print("{}\\{}".format(root, name))
