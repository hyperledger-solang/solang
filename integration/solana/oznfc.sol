contract Events {
    string public name;
    string public surname;

    event NameChanced(string message, string newName);
    event SurnameChanced(string message, string newSurname);

    constructor() public {
        name = 'myName';
        surname = 'mySurname';
    }

    function setName(string memory _name) public {
        name = _name;
        emit NameChanced('Name Chanced', 'x');
    }

    function setSurname(string memory _surname) public {
        surname = _surname;
        emit SurnameChanced('Surname Chanced', _surname);
    }

    function getNames() public view returns (string memory _name , string memory _surname ) {
        return (name ,surname);
    }


    function getName() public view returns (string memory _name ) {
        return name ;
    }

     function getSurname() public view returns (string memory _surname ) {
        return surname ;
    }

}