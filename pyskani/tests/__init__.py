from . import (
    test_ani,
    test_database
)

def load_tests(loader, suite, pattern):
    suite.addTests(loader.loadTestsFromModule(test_ani))
    suite.addTests(loader.loadTestsFromModule(test_database))
    return suite
