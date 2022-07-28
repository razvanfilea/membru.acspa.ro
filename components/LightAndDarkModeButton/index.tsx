import {ActionIcon, useMantineColorScheme} from "@mantine/core";
import {MdDarkMode, MdLightMode} from 'react-icons/md';

export default function LightAndDarkModeButton({}) {
    const {colorScheme, toggleColorScheme} = useMantineColorScheme();
    const dark = colorScheme === 'dark';

    return (
        <>
            <ActionIcon
                variant="filled"
                color={dark ? 'yellow' : 'blue'}
                onClick={() => toggleColorScheme()}
                title="Schimbă tema"
            >
                {dark ? <MdLightMode size={18}/> : <MdDarkMode size={18}/>}
            </ActionIcon>
        </>
    )
}
