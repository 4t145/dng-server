import os
os.popen('sqlite3')
os.popen('.open lexicon')
os.popen('sqlite3 create_index.sql')
os.popen('exit')